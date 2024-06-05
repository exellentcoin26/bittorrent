use std::str::FromStr;

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
            let decoded_value = serde_json::to_value(BencodeValue::from_str(&value)?)
                .context("failed to serialize bencode value to json")?;
            println!("{}", decoded_value);
        }
    }

    Ok(())
}
