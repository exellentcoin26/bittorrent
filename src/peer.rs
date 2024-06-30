use std::net::SocketAddrV4;

use anyhow::{bail, Context, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use self::message::{PeerHandShakePacket, PeerMessage};
use crate::util::{PeerId, Sha1Hash};

mod message;
mod piece;

pub use self::piece::PieceDescriptor;

pub struct Peer<C> {
    socket_addr: SocketAddrV4,
    connection: C,
}

pub struct Disconnected;
pub struct Connected {
    stream: TcpStream,
    peer_id: PeerId,
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

        read_bitfield(&mut stream).await?;

        stream
            .write_all(&PeerMessage::Interested.into_bytes())
            .await
            .context("sending peer interested message")?;

        read_unchoke(&mut stream).await?;

        Ok(Peer {
            socket_addr: self.socket_addr,
            connection: Connected {
                stream,
                peer_id: handshake_packet.peer_id,
            },
        })
    }
}

impl Peer<Connected> {
    pub fn peer_id(&self) -> &PeerId {
        &self.connection.peer_id
    }
}

async fn prepare_buffer_with_length(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let message_length = stream.read_u32().await.context("reading message length")?;
    Ok(vec![0u8; message_length as usize])
}

impl From<SocketAddrV4> for Peer<Disconnected> {
    fn from(value: SocketAddrV4) -> Self {
        Self::from_socket(value)
    }
}
