use anyhow::{bail, Context, Result};
use bytes::Bytes;
use tempfile::Builder as TempFileBuilder;
use tokio::net::TcpStream;

use super::{message::PeerMessage, prepare_buffer_with_length, Connected, Peer};
use crate::util::{hash_sha1, Sha1Hash};

const PIECE_BLOCK_SIZE: u32 = 16 * 1024;

pub struct PieceDescriptor {
    pub index: u32,
    pub length: u32,
    pub hash: Sha1Hash,
}

impl PieceDescriptor {
    pub fn new(index: u32, length: u32, hash: Sha1Hash) -> Self {
        Self {
            index,
            length,
            hash,
        }
    }
}

async fn read_piece_block(stream: &mut TcpStream) -> Result<PieceBlockResponse> {
    use tokio::io::AsyncReadExt;

    let mut buf = prepare_buffer_with_length(stream).await?;

    stream
        .read_exact(&mut buf)
        .await
        .context("reading piece block message")?;
    Ok(match PeerMessage::parse(buf.into()) {
        Ok(PeerMessage::Piece {
            index,
            begin,
            block,
        }) => PieceBlockResponse {
            index,
            begin,
            block,
        },
        Err(err) => return Err(err).context("parsing piece block message"),
        _ => bail!("unexpected peer message"),
    })
}

impl Peer<Connected> {
    pub async fn download_piece(
        &mut self,
        PieceDescriptor {
            index,
            length,
            hash,
        }: PieceDescriptor,
    ) -> Result<()> {
        use std::io::Write;
        use tokio::io::AsyncWriteExt;

        let stream = &mut self.connection.stream;

        // Request the piece.
        let mut buf = vec![0u8; length as usize];
        for req_block in generate_piece_block_requests(index, length) {
            // Request the block in the piece.
            stream
                .write_all(&req_block.to_message().into_bytes())
                .await
                .context("sending piece block request")?;

            // Receive the block.
            let rec_block = read_piece_block(stream)
                .await
                .context("reading piece block message")?;

            check_block_validity(&req_block, &rec_block)?;

            // Accumulate the values.
            buf[rec_block.begin as usize..(rec_block.begin + req_block.length) as usize]
                .copy_from_slice(&rec_block.block);
        }

        // Check the piece hash.
        if hash != hash_sha1(&buf) {
            bail!("piece hash does not match hash from torrent");
        }

        // Store piece on disk for now.
        let mut file = TempFileBuilder::new()
            .tempfile()
            .context("creating temporary file for piece")?;
        file.write_all(&buf).context("writing piece to tempfile")?;

        println!("Piece {index} downloaded to {}.", file.path().display());

        Ok(())
    }
}

struct PieceBlockRequest {
    index: u32,
    begin: u32,
    length: u32,
}

struct PieceBlockResponse {
    index: u32,
    begin: u32,
    block: Bytes,
}

fn generate_piece_block_requests(
    index: u32,
    length: u32,
) -> impl Iterator<Item = PieceBlockRequest> {
    let amount = (f64::from(length) / f64::from(PIECE_BLOCK_SIZE)).ceil() as usize;

    (0..amount).map(move |i| {
        let offset =
            u32::try_from(i * PIECE_BLOCK_SIZE as usize).expect("offset should fit in u32");
        let block_size = (length - offset).min(PIECE_BLOCK_SIZE);

        PieceBlockRequest {
            index,
            begin: offset,
            length: block_size,
        }
    })
}

impl PieceBlockRequest {
    pub(super) fn to_message(&self) -> PeerMessage {
        PeerMessage::Request {
            index: self.index,
            begin: self.begin,
            length: self.length,
        }
    }
}

fn check_block_validity(req: &PieceBlockRequest, res: &PieceBlockResponse) -> Result<()> {
    if res.index != req.index {
        bail!("received block piece index does not match requested index");
    }
    if res.begin != req.begin {
        bail!("received block piece offset does not match requested offset");
    }
    Ok(())
}
