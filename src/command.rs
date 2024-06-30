use std::{net::SocketAddrV4, path::PathBuf};

use bstr::BString;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
#[clap(rename_all = "snake_case")]
pub enum Command {
    /// Decode the given binary encoded value into a json value.
    Decode {
        value: BString,
    },
    Info {
        path: PathBuf,
    },
    Peers {
        path: PathBuf,
    },
    Handshake {
        path: PathBuf,
        peer: SocketAddrV4,
    },
    DownloadPiece {
        /// Path to the torrent file.
        path: PathBuf,
        /// Index of the piece to download.
        index: u32,
    },
    Download {
        /// Path to download the file to.
        #[arg(short)]
        output: PathBuf,
        /// Path to the torrent file.
        path: PathBuf,
    },
}
