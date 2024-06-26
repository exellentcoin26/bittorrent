use anyhow::{bail, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::util::{PeerId, Sha1Hash};

pub(super) enum PeerMessage {
    Unchoke,
    Interested,
    Bitfield,
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
    Piece {
        index: u32,
        begin: u32,
        block: Bytes,
    },
}

pub(super) struct PeerHandShakePacket {
    pub(super) info_hash: Sha1Hash,
    pub(super) peer_id: PeerId,
}

fn parse_empty(input: Bytes) -> Result<()> {
    if input.has_remaining() {
        bail!("bytes remaining when parsing empty remainder");
    }
    Ok(())
}

fn parse_ingore_payload(mut input: Bytes) -> Result<()> {
    // Consume all remaining bytes.
    input.advance(input.remaining());
    Ok(())
}

fn parse_request_payload(mut input: Bytes) -> Result<PeerMessage> {
    let index = input.get_u32();
    let begin = input.get_u32();
    let length = input.get_u32();

    if input.has_remaining() {
        bail!("bytes remaining when parsing request payload");
    }

    Ok(PeerMessage::Request {
        index,
        begin,
        length,
    })
}

fn parse_piece_payload(mut input: Bytes) -> Result<PeerMessage> {
    let index = input.get_u32();
    let begin = input.get_u32();

    Ok(PeerMessage::Piece {
        index,
        begin,
        block: input,
    })
}

impl PeerMessage {
    pub(super) fn parse(mut input: Bytes) -> Result<Self> {
        let message_id = input.get_u8();

        Ok(match message_id {
            1 => {
                parse_empty(input)?;
                PeerMessage::Unchoke
            }
            2 => {
                parse_empty(input)?;
                PeerMessage::Interested
            }
            5 => {
                parse_ingore_payload(input)?;
                PeerMessage::Bitfield
            }
            6 => parse_request_payload(input)?,
            7 => parse_piece_payload(input)?,
            _ => bail!("unhandled message id: {}", message_id),
        })
    }

    pub(super) fn into_bytes(self) -> Bytes {
        let mut buf = BytesMut::new();

        match self {
            PeerMessage::Unchoke => buf.put_u8(1),
            PeerMessage::Interested => buf.put_u8(2),
            PeerMessage::Request {
                index,
                begin,
                length,
            } => {
                buf.put_u8(6);
                buf.put_u32(index);
                buf.put_u32(begin);
                buf.put_u32(length);
            }
            PeerMessage::Piece {
                index,
                begin,
                block,
            } => {
                buf.put_u8(7);
                buf.put_u32(index);
                buf.put_u32(begin);
                buf.put(block);
            }

            PeerMessage::Bitfield => unimplemented!("message unsupported for serialization"),
        }

        buf.freeze()
    }
}

impl PeerHandShakePacket {
    pub(super) fn new(info_hash: Sha1Hash, peer_id: PeerId) -> Self {
        Self { info_hash, peer_id }
    }

    pub(super) fn parse(mut input: Bytes) -> Result<Self> {
        let header_length = input.get_u8();
        let header = input.copy_to_bytes(header_length as usize);

        if header != b"BitTorrent protocol".as_slice() {
            bail!("Unexpected peer handshake packet.");
        }

        // Reserved zero-bytes.
        input.get_u64();

        let info_hash = input.copy_to_bytes(20);
        let peer_id = input.copy_to_bytes(20);

        Ok(PeerHandShakePacket {
            info_hash: *info_hash
                .first_chunk()
                .expect("info hash should be 20 bytes"),
            peer_id: *peer_id.first_chunk().expect("peer id should be 20 bytes"),
        })
    }

    pub(super) fn into_bytes(self) -> Bytes {
        use std::fmt::Write;

        let prepare = || -> Result<Bytes, std::fmt::Error> {
            let mut buf = BytesMut::with_capacity(68);
            buf.put_u8(19);
            buf.write_str("BitTorrent protocol")?;
            buf.put_u64(0);
            buf.extend(self.info_hash);
            buf.extend(self.peer_id);

            Ok(buf.freeze())
        };

        prepare().expect("prepared peer handshake buffer should not be empty")
    }
}
