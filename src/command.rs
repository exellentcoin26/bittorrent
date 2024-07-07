use std::{net::SocketAddrV4, path::PathBuf};

use anyhow::{Context, Result};
use bencode::BencodeValue;
use bstr::BString;
use clap::{Parser, Subcommand};

use crate::{
    downloader::TorrentDownloader,
    peer::{Peer, PieceDescriptor},
    torrent::Torrent,
    tracker::Tracker,
    util::calculate_piece_length,
};

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
        /// Path to download the piece to.
        #[arg(short)]
        output: PathBuf,
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

impl Command {
    pub async fn execute(self) -> Result<()> {
        match self {
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
                println!("{}", tracker_response.peers);
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
            } => download_piece(output, path, index).await?,
            Command::Download { output, path } => {
                let torrent =
                    Torrent::from_file_path(&path).context("reading torrent from file path")?;

                TorrentDownloader::new(torrent)
                    .await
                    .context("initializing downloader")?
                    .download_to_location(&output)
                    .await
                    .context("downloading torrent")?;

                println!("Downloaded {} to {}", path.display(), output.display());
            }
        }

        Ok(())
    }
}

async fn download_piece(output: PathBuf, path: PathBuf, index: u32) -> Result<()> {
    use std::io::Write;

    let torrent = Torrent::from_file_path(path).context("reading torrent from file path")?;
    let tracker = Tracker::from(&torrent);

    // Use first peer found.
    let peer_socket_addr = *tracker
        .poll()
        .await
        .context("polling tracker")?
        .peers
        .first()
        .context("no peer found")?;

    let mut peer = Peer::from_socket(peer_socket_addr)
        .handshake(*tracker.info_hash(), *tracker.peer_id())
        .await
        .context("performing peer handshake")?;

    let piece_hash = torrent
        .info
        .pieces
        .get(index as usize)
        .context("piece index outside range")?;
    let piece = peer
        .download_piece(PieceDescriptor::new(
            index,
            calculate_piece_length(torrent.info.piece_length, torrent.info.length, index),
            *piece_hash,
        ))
        .await
        .context("downloading a single piece")?;

    let mut file =
        std::fs::File::create(&output).context("creating file to download torrent piece to")?;
    file.write_all(&piece)
        .context("writing to torrent piece output file")?;

    println!("Piece {index} downloaded to {}.", output.display());

    Ok(())
}
