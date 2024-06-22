use anyhow::{bail, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::util::{InfoHash, PeerId};

enum PeerMessage {
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

impl PeerMessage {
    pub fn parse(mut input: Bytes) -> Result<Self> {
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

    pub fn into_bytes(self) -> Bytes {
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
