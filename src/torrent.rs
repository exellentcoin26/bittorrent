use std::path::Path;

use anyhow::{Context, Result};
use bencode::BencodeValue;
use bstr::BString;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::util::{hash_sha1, serde_with::ArrayChunksWithLength, Sha1Hash};

#[derive(Debug)]
pub struct Torrent {
    pub announce: String,
    pub info: TorrentInfo,
    pub info_hash: Sha1Hash,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct TorrentInfo {
    pub length: u64,
    pub name: BString,
    #[serde(rename = "piece length")]
    pub piece_length: u32,
    #[serde_as(as = "ArrayChunksWithLength<20>")]
    pub pieces: Vec<Sha1Hash>,
}

#[derive(Debug, Clone, Copy)]
pub struct TorrentOverview<'a> {
    tracker_url: &'a str,
    length: usize,
    info_hash: &'a Sha1Hash,
    piece_length: usize,
    pieces: &'a [Sha1Hash],
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
                    format!("opening torrent file from path `{:?}`", path.as_ref())
                })?;

                let contents = {
                    let mut content_buf = match file.metadata() {
                        Ok(m) => Vec::with_capacity(m.len() as usize),
                        _ => Vec::new(),
                    };
                    file.read_to_end(&mut content_buf)
                        .context("reading contents of torrent file")?;
                    content_buf
                };

                let parsed_contents =
                    BencodeValue::try_from_bytes(&contents).context("decoding torrent contents")?;

                parsed_contents
                    .into_deserialize()
                    .context("torrent contents do not match torrent specifications")
            }

            fn torrent_info_hash(&self) -> Result<Sha1Hash> {
                let torrent_info_bencode_bytes = &*BencodeValue::from_serialize(&self.info)
                    .context("serializing torrent info")?
                    .to_byte_string()
                    .context("serializing bencode value as bytes")?;

                Ok(hash_sha1(torrent_info_bencode_bytes))
            }
        }

        let file = TorrentFile::from_file_path(path)?;

        let info_hash = file
            .torrent_info_hash()
            .context("calculating torrent info hash")?;

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
