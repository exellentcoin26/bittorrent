use anyhow::{Context, Result};
use clap::Parser;

use crate::{
    bencode::BencodeValue,
    command::{Cli, Command},
};

mod bencode;
mod command;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Decode { value } => {
            let decoded_value = serde_json::to_value(BencodeValue::try_from_bytes(&value)?)
                .context("failed to serialize bencode value to json")?;
            println!("{}", decoded_value);
        }
    }

    Ok(())
}
