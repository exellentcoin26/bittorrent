use std::{net::SocketAddrV4, time::Duration};

use anyhow::{Context, Result};
use bytes::Bytes;
use serde::Serialize;
use serde_with::{serde_as, Bytes as SerdeBytes, FromInto};

#[derive(Debug)]
pub struct Tracker {
    url: String,
    info_hash: Bytes,
    peer_id: [u8; 20],
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
}

#[serde_as]
#[derive(Serialize)]
struct TrackerRequest {
    info_hash: Bytes,
    #[serde_as(as = "SerdeBytes")]
    peer_id: [u8; 20],
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    #[serde_as(as = "FromInto<u8>")]
    compact: bool,
}

#[derive(Debug)]
pub struct TrackerResponse {
    interval: Duration,
    peers: Vec<SocketAddrV4>,
}

impl Tracker {
    pub fn new(announce: String, info_hash: Bytes, size: u64) -> Self {
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

    pub fn poll(&self) -> Result<TrackerResponse> {
        mod inner {
            use std::{
                net::{Ipv4Addr, SocketAddrV4},
                time::Duration,
            };

            use anyhow::{bail, Result};
            use bytes::Bytes;
            use serde::Deserialize;
            use serde_with::{serde_as, DurationSeconds};

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
                                u16::from_le_bytes(*port_bytes),
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?;

                    Ok(Self { interval, peers })
                }
            }
        }

        let response = reqwest::blocking::get(&self.url)
            .context("requesting tracker announce url failed")?
            .json::<inner::TrackerResponse>()
            .context("failed to deserialize tracker announce response")?;

        TrackerResponse::try_from(response)
    }
}
