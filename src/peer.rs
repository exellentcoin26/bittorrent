use std::net::SocketAddrV4;

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use tempfile::Builder as TempFileBuilder;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use self::message::{PeerHandShakePacket, PeerMessage};
use crate::util::{hash_sha1, PeerId, Sha1Hash};

mod message;

const PIECE_BLOCK_SIZE: u32 = 16 * 1024;

pub struct Peer<C> {
    socket_addr: SocketAddrV4,
    connection: C,
}

pub struct Disconnected;
pub struct Connected {
    stream: TcpStream,
    peer_id: PeerId,
}

impl Peer<Disconnected> {
    pub fn from_socket(socket: SocketAddrV4) -> Self {
        Self {
            socket_addr: socket,
            connection: Disconnected,
        }
    }

    pub async fn handshake(
        self,
        info_hash: Sha1Hash,
        client_peer_id: PeerId,
    ) -> Result<Peer<Connected>> {
        let mut stream = TcpStream::connect(self.socket_addr)
            .await
            .context("connecting to peer")?;

        stream
            .write_all(&PeerHandShakePacket::new(info_hash, client_peer_id).into_bytes())
            .await
            .context("sending handshake packet")?;

        let mut buf = Box::new([0u8; 68]) as Box<[u8]>;
        stream
            .read_exact(&mut buf)
            .await
            .context("reading handshake response packet")?;
        let handshake_packet =
            PeerHandShakePacket::parse(buf.into()).context("parsing peer handshake packet")?;

        if handshake_packet.info_hash != info_hash {
            bail!("info hash received from handshake does not match");
        }

        Ok(Peer {
            socket_addr: self.socket_addr,
            connection: Connected {
                stream,
                peer_id: handshake_packet.peer_id,
            },
        })
    }
}

async fn prepare_buffer_with_length(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let message_length = stream.read_u32().await.context("reading message length")?;
    Ok(vec![0u8; message_length as usize])
}

async fn read_bitfield(stream: &mut TcpStream) -> Result<()> {
    let mut buf = prepare_buffer_with_length(stream).await?;

    stream
        .read_exact(&mut buf)
        .await
        .context("reading bitfield message")?;
    match PeerMessage::parse(buf.into()) {
        Ok(PeerMessage::Bitfield) => (),
        Err(err) => return Err(err).context("parsing peer bitfield message"),
        _ => bail!("unexpected peer message"),
    }
    Ok(())
}

async fn read_unchoke(stream: &mut TcpStream) -> Result<()> {
    let mut buf = prepare_buffer_with_length(stream).await?;

    stream
        .read_exact(&mut buf)
        .await
        .context("reading unchoke message")?;
    match PeerMessage::parse(buf.into()) {
        Ok(PeerMessage::Unchoke) => (),
        Err(err) => return Err(err).context("parsing unchoke message"),
        _ => bail!("unexpected peer message"),
    }
    Ok(())
}

async fn read_piece_block(stream: &mut TcpStream) -> Result<PieceBlockResponse> {
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
    pub fn peer_id(&self) -> &PeerId {
        &self.connection.peer_id
    }

    pub async fn download_piece(&mut self, index: u32, length: u32, hash: Sha1Hash) -> Result<()> {
        use std::io::Write;

        let stream = &mut self.connection.stream;

        // Receive bitfield message.
        read_bitfield(stream).await?;

        // Send interested message.
        stream
            .write_all(&PeerMessage::Interested.into_bytes())
            .await
            .context("sending peer interested message")?;

        // Receive unchoke message.
        read_unchoke(stream).await?;

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

            if rec_block.index != req_block.index {
                bail!("received block piece index does not match requested index");
            }
            if rec_block.begin != req_block.begin {
                bail!("received block piece offset does not match requested offset");
            }

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
            .prefix(&format!("torrent-piece-{index}"))
            .tempfile()
            .context("creating temporary file for piece")?;
        file.write_all(&buf).context("writing piece to tempfile")?;

        println!("Piece {index} downloaded to {}.", file.path().display());

        Ok(())
    }
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

impl PieceBlockRequest {
    fn to_message(&self) -> PeerMessage {
        PeerMessage::Request {
            index: self.index,
            begin: self.begin,
            length: self.length,
        }
    }
}

impl From<SocketAddrV4> for Peer<Disconnected> {
    fn from(value: SocketAddrV4) -> Self {
        Self::from_socket(value)
    }
}
