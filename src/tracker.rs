use std::{borrow::Cow, net::SocketAddrV4, time::Duration};

use anyhow::{Context, Result};
use bencode::BencodeValue;
use bstr::BString;
use serde::Serialize;
use serde_with::{serde_as, FromInto};

use crate::{
    torrent::Torrent,
    util::{PeerId, Sha1Hash},
};

#[derive(Debug)]
pub struct Tracker {
    url: String,
    info_hash: Sha1Hash,
    peer_id: PeerId,
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
}

#[serde_as]
#[derive(Debug, Serialize)]
struct TrackerRequest {
    /// Iso 8859-1 decoded byte string (needed to smuggle random bytes into url encoder).
    info_hash: String,
    /// Iso 8859-1 decoded byte string (needed to smuggle random bytes into url encoder).
    peer_id: String,
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    #[serde_as(as = "FromInto<u8>")]
    compact: bool,
}

#[derive(Debug)]
pub struct TrackerResponse {
    pub interval: Duration,
    pub peers: Peers,
}

#[derive(Debug, Clone)]
pub struct Peers(pub Vec<SocketAddrV4>);

impl From<&Torrent> for Tracker {
    fn from(value: &Torrent) -> Self {
        Self::new(value.announce.clone(), value.info_hash, value.info.length)
    }
}

impl Tracker {
    pub fn new(announce: String, info_hash: Sha1Hash, size: u64) -> Self {
        Self {
            url: announce,
            info_hash,
            peer_id: rand::random(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: size,
        }
    }

    pub async fn poll(&self) -> Result<TrackerResponse> {
        let query = TrackerRequest {
            info_hash: decode_iso_8859_1(&self.info_hash),
            peer_id: decode_iso_8859_1(&self.peer_id),
            port: self.port,
            uploaded: self.uploaded,
            downloaded: self.downloaded,
            left: self.left,
            compact: true,
        };

        query.send(&self.url).await.context("polling tracker")
    }

    pub fn info_hash(&self) -> &Sha1Hash {
        &self.info_hash
    }

    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }
}

impl TrackerRequest {
    pub async fn send(self, url: &str) -> Result<TrackerResponse> {
        tracing::debug!("Sending request to tracker");

        mod inner {
            use std::{
                net::{Ipv4Addr, SocketAddrV4},
                time::Duration,
            };

            use anyhow::{bail, Result};
            use bytes::Bytes;
            use serde::Deserialize;
            use serde_with::{serde_as, DurationSeconds};

            use super::Peers;

            #[serde_as]
            #[derive(Debug, Deserialize)]
            pub(super) struct TrackerResponse {
                #[serde_as(as = "DurationSeconds")]
                interval: Duration,
                peers: Bytes,
            }

            impl TryFrom<TrackerResponse> for super::TrackerResponse {
                type Error = anyhow::Error;

                fn try_from(value: TrackerResponse) -> Result<Self> {
                    let TrackerResponse { interval, peers } = value;
                    let peers = peers
                        .chunks(6)
                        .map(|c| {
                            let Some((ip_bytes, port_bytes)) = c
                                .split_first_chunk::<4>()
                                .and_then(|(ib, c)| c.first_chunk::<2>().map(|pb| (ib, pb)))
                            else {
                                bail!("peers array entry not of length 6 bytes");
                            };

                            Ok(SocketAddrV4::new(
                                Ipv4Addr::from(*ip_bytes),
                                u16::from_be_bytes(*port_bytes),
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?;

                    Ok(Self {
                        interval,
                        peers: Peers(peers),
                    })
                }
            }
        }

        let response_bytes = BString::from_iter(
            reqwest::get(format!("{url}?{}", url_encode(self)?))
                .await
                .context("requesting tracker announce url")?
                .bytes()
                .await
                .context("reading tracker announce response bytes")?,
        );

        let response: inner::TrackerResponse = BencodeValue::try_from_bytes(&response_bytes)
            .context("parsing tracker announce response as bencode value")?
            .into_deserialize()
            .context("deserializing tracker announce response")?;

        TrackerResponse::try_from(response)
    }
}

impl std::fmt::Display for Peers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for peer in self.0.iter() {
            writeln!(f, "{}", peer)?;
        }
        Ok(())
    }
}

impl std::ops::Deref for Peers {
    type Target = [SocketAddrV4];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Peers {
    pub fn into_socket_addrs(self) -> Vec<SocketAddrV4> {
        self.0
    }
}

/// Adapted from [https://github.com/nox/serde_urlencoded/pull/60/files]
fn url_encode(input: impl Serialize) -> Result<String> {
    use form_urlencoded::Serializer as UrlEncoder;
    use serde_urlencoded::Serializer as UrlEncodeSerializer;

    let mut urlencoder = UrlEncoder::new(String::new());
    urlencoder.encoding_override(Some(&encode_iso_8859_1));
    input
        .serialize(UrlEncodeSerializer::new(&mut urlencoder))
        .context("urlencoding input")?;
    Ok(urlencoder.finish())
}

fn encode_iso_8859_1(input: &str) -> Cow<[u8]> {
    input
        .chars()
        .map(|c| u8::try_from(u32::from(c)).expect("utf-8 character"))
        .collect()
}

fn decode_iso_8859_1(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| char::from(*byte)).collect()
}
