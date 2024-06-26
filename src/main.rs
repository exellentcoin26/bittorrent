use anyhow::{Context, Result};
use bencode::BencodeValue;
use clap::Parser;

use crate::{
    command::{Cli, Command},
    peer::Peer,
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
        Command::DownloadPiece {
            output,
            path,
            index,
        } => {
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

            peer.download_piece(0, torrent.info.piece_length, torrent.info.pieces[0])
                .await
                .context("downloading a single piece")?;
        }
    }

    Ok(())
}
