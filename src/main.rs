use anyhow::Result;
use clap::Parser;

use crate::command::Cli;

mod command;
mod downloader;
mod peer;
mod torrent;
mod tracker;
mod util;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.command.execute().await
}
