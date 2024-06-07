use std::path::Path;

use anyhow::{Context, Result};
use bstr::{BStr, BString};
use serde::{Deserialize, Serialize};

use crate::bencode::BencodeValue;

#[derive(Debug, Serialize, Deserialize)]
pub struct Torrent {
    pub announce: BString,
    pub info: TorrentInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TorrentInfo {
    pub length: u64,
    pub name: BString,
    #[serde(rename = "piece length")]
    pub piece_length: u64,
    pub pieces: BString,
}

#[derive(Debug, Clone, Copy)]
pub struct TorrentOverview<'a> {
    tracker_url: &'a BStr,
    length: usize,
}

impl Torrent {
    pub fn from_file_path(path: impl AsRef<Path>) -> Result<Self> {
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

        let parsed_contents =
            BencodeValue::try_from_bytes(&contents).context("failed to decode torrent contents")?;

        let parsed_contents =
            serde_json::to_value(parsed_contents).context("failed to decode torrent contents")?;

        serde_json::from_value(parsed_contents)
            .context("torrent contents do not match torrent specifications")
    }

    pub fn overview(&self) -> TorrentOverview {
        TorrentOverview {
            tracker_url: self.announce.as_ref(),
            length: self.info.length as usize,
        }
    }
}

impl std::fmt::Display for TorrentOverview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Tracker URL: {}", self.tracker_url)?;
        writeln!(f, "length: {}", self.length)
    }
}
