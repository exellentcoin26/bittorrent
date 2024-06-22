use std::{net::SocketAddrV4, path::PathBuf};

use bstr::BString;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
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
}
