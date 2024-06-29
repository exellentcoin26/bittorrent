use anyhow::{Context, Result};
use bencode::BencodeValue;
use clap::Parser;

use crate::{
    command::{Cli, Command},
    peer::{piece::PieceDescriptor, Peer},
    torrent::Torrent,
    tracker::Tracker,
};

mod command;
mod peer;
mod torrent;
mod tracker;
mod util;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Decode { value } => {
            let decoded_value = serde_json::to_value(BencodeValue::try_from_bytes(&value)?)
                .context("serializing bencode value to json")?;
            println!("{}", decoded_value);
        }
        Command::Info { path } => {
            let torrent = Torrent::from_file_path(path).context("reading torrent from path")?;
            println!("{}", torrent.overview());
        }
        Command::Peers { path } => {
            let torrent =
                Torrent::from_file_path(path).context("reading torrent from file path")?;
            let tracker = Tracker::from(&torrent);

            let tracker_response = tracker.poll().await.context("polling tracker")?;
            println!("{}", tracker_response.peers());
        }
        Command::Handshake { path, peer } => {
            let torrent =
                Torrent::from_file_path(path).context("reading torrent from file path")?;
            let tracker = Tracker::from(&torrent);

            let peer = Peer::from_socket(peer)
                .handshake(*tracker.info_hash(), *tracker.peer_id())
                .await
                .context("performing peer handshake")?;
            println!("Peer ID: {}", hex::encode(peer.peer_id()))
        }
        Command::DownloadPiece { path, index } => {
            let torrent =
                Torrent::from_file_path(path).context("reading torrent from file path")?;
            let tracker = Tracker::from(&torrent);

            // Use first peer found.
            let peer_socket_addr = *tracker
                .poll()
                .await
                .context("polling tracker")?
                .peers()
                .first()
                .context("no peer found")?;

            let mut peer = Peer::from_socket(peer_socket_addr)
                .handshake(*tracker.info_hash(), *tracker.peer_id())
                .await
                .context("performing peer handshake")?;

            let piece_hash = torrent
                .info
                .pieces
                .get(index as usize)
                .context("piece index outside range")?;
            peer.download_piece(PieceDescriptor::new(
                index,
                calculate_piece_length(torrent.info.piece_length, torrent.info.length, index)?,
                *piece_hash,
            ))
            .await
            .context("downloading a single piece")?;
        }
    }

    Ok(())
}

fn calculate_piece_length(piece_length: u32, torrent_length: u64, piece_index: u32) -> Result<u32> {
    Ok(piece_length.min(
        u32::try_from(torrent_length - u64::from(piece_index * piece_length))
            .context("piece length should fit in 32 bits")?,
    ))
}
