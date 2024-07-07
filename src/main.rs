use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::command::Cli;

mod command;
mod downloader;
mod peer;
mod torrent;
mod tracker;
mod util;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    cli.command.execute().await
}
