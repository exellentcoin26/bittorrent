use std::{
    fmt::Write,
    net::{Ipv4Addr, SocketAddrV4},
};

use anyhow::{Context, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::util::{InfoHash, PeerId};

pub struct Peer(Ipv4Addr, u16);

impl Peer {
    pub fn from_socket(socket: SocketAddrV4) -> Self {
        Self(*socket.ip(), socket.port())
    }

    pub async fn handshake(&self, info_hash: &InfoHash, client_peer_id: &PeerId) -> Result<()> {
        let mut stream = TcpStream::connect((self.0, self.1))
            .await
            .context("failed to connect to peer")?;

        stream
            .write_all(&prepare_peer_handshake_packet(info_hash, client_peer_id))
            .await
            .context("sending handshake packet")?;

        let mut buf = vec![0u8; 68];
        stream
            .read_exact(&mut buf)
            .await
            .context("reading handshake response packet")?;
        let handshake_packet = parse_peer_handshake_packet(buf.into());
        println!("Peer ID: {}", hex::encode(handshake_packet.peer_id));

        Ok(())
    }
}

impl From<SocketAddrV4> for Peer {
    fn from(value: SocketAddrV4) -> Self {
        Self::from_socket(value)
    }
}

struct PeerHandShakePacket {
    info_hash: InfoHash,
    peer_id: PeerId,
}

fn prepare_peer_handshake_packet(info_hash: &InfoHash, client_peer_id: &PeerId) -> Bytes {
    let prepare = || -> Result<Bytes, std::fmt::Error> {
        let mut buf = BytesMut::with_capacity(68);
        buf.put_u8(19);
        buf.write_str("BitTorrent protocol")?;
        buf.put_u64(0);
        buf.extend(info_hash);
        buf.extend(client_peer_id);

        Ok(buf.freeze())
    };

    prepare().expect("prepared peer handshake buffer should not be empty")
}

fn parse_peer_handshake_packet(mut input: Bytes) -> PeerHandShakePacket {
    let header_length = input.get_u8();
    let header = input.copy_to_bytes(header_length as usize);

    if header != b"BitTorrent protocol".as_slice() {
        panic!("Unexpected peer handshake packet!");
    }

    // Reserved zero-bytes.
    input.get_u64();

    let info_hash = input.copy_to_bytes(20);
    let peer_id = input.copy_to_bytes(20);

    PeerHandShakePacket {
        info_hash: *info_hash
            .first_chunk()
            .expect("info hash should be 20 bytes"),
        peer_id: *peer_id.first_chunk().expect("peer id should be 20 bytes"),
    }
}
