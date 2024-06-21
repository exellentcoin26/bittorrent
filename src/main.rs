use anyhow::{Context, Result};
use bencode::BencodeValue;
use clap::Parser;

use crate::{
    command::{Cli, Command},
    torrent::Torrent,
    tracker::Tracker,
};

mod command;
mod torrent;
mod tracker;
mod util;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Decode { value } => {
            let decoded_value = serde_json::to_value(BencodeValue::try_from_bytes(&value)?)
                .context("failed to serialize bencode value to json")?;
            println!("{}", decoded_value);
        }
        Command::Info { path } => {
            let torrent =
                Torrent::from_file_path(path).context("reading torrent from path failed")?;
            println!("{}", torrent.overview());
        }
        Command::Peers { path } => {
            let torrent =
                Torrent::from_file_path(path).context("reading torrent from file path failed")?;
            let tracker = Tracker::from(torrent);

            let tracker_response = tracker.poll().await.context("failed to poll tracker")?;
            println!("{}", tracker_response.peers());
        }
    }

    Ok(())
}
