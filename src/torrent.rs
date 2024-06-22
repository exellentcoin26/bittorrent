use std::path::Path;

use anyhow::{Context, Result};
use bencode::BencodeValue;
use bstr::BString;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::util::{serde_with::ByteChunksWithLength, InfoHash};

#[derive(Debug)]
pub struct Torrent {
    pub announce: String,
    pub info: TorrentInfo,
    pub info_hash: InfoHash,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct TorrentInfo {
    pub length: u64,
    pub name: BString,
    #[serde(rename = "piece length")]
    pub piece_length: u64,
    #[serde_as(as = "ByteChunksWithLength<20>")]
    pub pieces: Vec<Bytes>,
}

#[derive(Debug, Clone, Copy)]
pub struct TorrentOverview<'a> {
    tracker_url: &'a str,
    length: usize,
    info_hash: &'a InfoHash,
    piece_length: usize,
    pieces: &'a [Bytes],
}

impl Torrent {
    pub fn from_file_path(path: impl AsRef<Path>) -> Result<Self> {
        #[derive(Debug, Deserialize)]
        struct TorrentFile {
            pub announce: String,
            pub info: TorrentInfo,
        }

        impl TorrentFile {
            fn from_file_path(path: impl AsRef<Path>) -> Result<Self> {
                use std::io::Read;

                let mut file = std::fs::File::open(&path).with_context(|| {
                    format!(
                        "failed to open torrent file from path `{:?}`",
                        path.as_ref()
                    )
                })?;

                let contents = {
                    let mut content_buf = match file.metadata() {
                        Ok(m) => Vec::with_capacity(m.len() as usize),
                        _ => Vec::new(),
                    };
                    file.read_to_end(&mut content_buf)
                        .context("failed to read contents of torrent file")?;
                    content_buf
                };

                let parsed_contents = BencodeValue::try_from_bytes(&contents)
                    .context("failed to decode torrent contents")?;

                parsed_contents
                    .into_deserialize()
                    .context("torrent contents do not match torrent specifications")
            }

            fn torrent_info_hash(&self) -> Result<InfoHash> {
                use sha1::{Digest, Sha1};

                let torrent_info_bencode_bytes = &*BencodeValue::from_serialize(&self.info)
                    .context("failed to serialize torrent info")?
                    .to_byte_string()
                    .context("failed to serialize bencode value as bytes")?;

                let mut hasher = Sha1::new();
                hasher.update(torrent_info_bencode_bytes);
                Ok(hasher.finalize().into())
            }
        }

        let file = TorrentFile::from_file_path(path)?;

        let info_hash = file
            .torrent_info_hash()
            .context("failed to calculate torrent info hash")?;

        Ok(Self {
            announce: file.announce,
            info: file.info,
            info_hash,
        })
    }

    pub fn overview(&self) -> TorrentOverview {
        TorrentOverview {
            tracker_url: self.announce.as_ref(),
            length: self.info.length as usize,
            info_hash: &self.info_hash,
            piece_length: self.info.piece_length as usize,
            pieces: &self.info.pieces,
        }
    }
}

impl std::fmt::Display for TorrentOverview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Tracker URL: {}", self.tracker_url)?;
        writeln!(f, "length: {}", self.length)?;
        writeln!(f, "Info Hash: {}", hex::encode(self.info_hash))?;
        writeln!(f, "Piece Length: {}", self.piece_length)?;
        writeln!(f, "Piece Hashes:")?;
        for piece in self.pieces {
            writeln!(f, "{}", hex::encode(piece))?;
        }
        Ok(())
    }
}
