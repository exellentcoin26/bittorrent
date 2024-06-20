use anyhow::{Context, Result};
use bencode::BencodeValue;
use clap::Parser;

use crate::{
    command::{Cli, Command},
    torrent::Torrent,
};

mod command;
mod torrent;
mod tracker;
mod util;

fn main() -> Result<()> {
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
            println!("{}", torrent.overview())
        }
    }

    Ok(())
}
