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
            let tracker = Tracker::from(torrent);

            let tracker_response = tracker.poll().await.context("polling tracker")?;
            println!("{}", tracker_response.peers());
        }
        Command::Handshake { path, peer } => {
            let torrent =
                Torrent::from_file_path(path).context("reading torrent from file path")?;
            let tracker = Tracker::from(torrent);

            let peer = Peer::from_socket(peer)
                .handshake(tracker.info_hash(), tracker.peer_id())
                .await
                .context("performing peer handshake")?;
            println!("Peer ID: {}", hex::encode(peer.peer_id()))
        }
    }

    Ok(())
}
