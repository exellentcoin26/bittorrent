use std::{
    collections::{HashSet, VecDeque},
    net::SocketAddrV4,
    path::Path,
    time::Duration,
};

use anyhow::{Context, Result};
use tokio::{
    sync::{mpsc, watch},
    task::{JoinHandle, JoinSet},
};

use crate::{
    peer::{Connected, Peer, PieceDescriptor},
    torrent::Torrent,
    tracker::{Peers, Tracker, TrackerResponse},
    util::Sha1Hash,
    util::{calculate_piece_length, PeerId},
};

const MAX_CONCURRENT_DOWNLOADS: u32 = 20;

pub struct TorrentDownloader {
    piece_queue: VecDeque<PieceDescriptor>,
    tracker: Tracker,
    client_peer_id: PeerId,
}

fn generate_piece_queue(
    piece_hashes: Vec<Sha1Hash>,
    piece_length: u32,
    torrent_length: u64,
) -> VecDeque<PieceDescriptor> {
    let piece_descriptors = {
        use rand::seq::SliceRandom;

        let mut rng = rand::thread_rng();
        let mut piece_descriptors = piece_hashes
            .into_iter()
            .enumerate()
            .map(|(index, piece_hash)| {
                let index = u32::try_from(index).expect("piece index should fit in 32 bits");
                PieceDescriptor::new(
                    index,
                    calculate_piece_length(piece_length, torrent_length, index),
                    piece_hash,
                )
            })
            .collect::<Vec<_>>();

        piece_descriptors.shuffle(&mut rng);
        piece_descriptors
    };

    VecDeque::from_iter(piece_descriptors)
}

fn spawn_tracker_poller(
    tracker: Tracker,
    tracker_tx: watch::Sender<Option<Peers>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut last_interval = None;

        // Close this loop using task aborting.
        loop {
            println!("Polling tracker");
            let TrackerResponse { peers, interval } = match tracker.poll().await {
                Ok(res) => res,
                Err(err) => {
                    eprintln!("{}", err);

                    if let Some(last_interval) = last_interval {
                        println!("Failed to poll tracker");
                        tokio::time::sleep(last_interval).await;
                    }
                    continue;
                }
            };

            dbg!(interval);
            last_interval = Some(interval);

            println!("Sending value");
            tracker_tx.send_replace(Some(peers));
            println!("Sent peers and going to sleep");
            tokio::time::sleep(interval).await;
        }
    })
}

async fn fetch_new_peers<'a>(
    active_peers: &'a HashSet<SocketAddrV4>,
    tracker_rx: &mut watch::Receiver<Option<Peers>>,
) -> Option<impl Iterator<Item = SocketAddrV4> + 'a> {
    let Some(usable_peers) = tracker_rx.borrow_and_update().clone() else {
        tokio::time::sleep(Duration::from_millis(100)).await;
        return None;
    };

    Some(
        usable_peers
            .into_socket_addrs()
            .into_iter()
            .filter(|p| !active_peers.contains(p)),
    )
}

fn spawn_piece_download_task(
    peer_socket_addr: SocketAddrV4,
    piece_des: PieceDescriptor,
    info_hash: Sha1Hash,
    client_peer_id: PeerId,
    handles: &mut JoinSet<PieceDownloadResult>,
) {
    handles.spawn(async move {
        let peer = match Peer::from_socket(peer_socket_addr)
            .handshake(info_hash, client_peer_id)
            .await
        {
            Ok(p) => p,
            Err(_) => {
                return PieceDownloadResult::Error {
                    peer_socket_addr,
                    piece_des,
                }
            }
        };

        PieceDownloadResult::Success {
            peer,
            piece: (piece_des, Vec::new()),
        }
    });
}

impl TorrentDownloader {
    pub async fn new(
        torrent: Torrent,
        // peer_socket_addresses: impl IntoIterator<Item = SocketAddrV4>,
        // client_peer_id: PeerId,
    ) -> Result<Self> {
        let tracker = Tracker::from(&torrent);

        let client_peer_id = *tracker.peer_id();

        let torrent_length = torrent.info.length;
        let piece_length = torrent.info.piece_length;
        let piece_hashes = torrent.info.pieces;

        let piece_queue = generate_piece_queue(piece_hashes, piece_length, torrent_length);

        Ok(Self {
            piece_queue,
            tracker,
            client_peer_id,
        })
    }

    pub async fn download(mut self, _output_location: impl AsRef<Path>) -> Result<()> {
        // For every peer available to download, start a task that downloads and returns the piece.
        // If a new peer becomes available, immediatly pick up a new task.
        // A peer should be donated to a task and returned when done downloading that piece.
        //
        // # Idea
        //
        // Tracker polling task that inserts peers as they become available.
        // Main loop -> Checks channel for new peers, creates tasks and checks if tasks have been
        // completed.

        let mut handles = JoinSet::new();

        let info_hash = *self.tracker.info_hash();
        let client_peer_id = *self.tracker.peer_id();

        let (tracker_tx, mut tracker_rx) = watch::channel(None);
        let mut active_peers: HashSet<SocketAddrV4> = HashSet::new();

        let tracker_handle = spawn_tracker_poller(self.tracker, tracker_tx);

        'main: loop {
            println!("Doing next iteration");

            let Some(new_peers) = fetch_new_peers(&active_peers, &mut tracker_rx).await else {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };

            let mut new_active_peers = HashSet::new();
            // Start a task for every peer that is inactive.
            for peer in new_peers {
                let piece_des = match self.piece_queue.pop_front() {
                    Some(p) => p,
                    None => break 'main,
                };

                println!("Taking piece descriptor from the queue");

                spawn_piece_download_task(peer, piece_des, info_hash, client_peer_id, &mut handles);

                new_active_peers.insert(peer);
            }

            active_peers.extend(new_active_peers);

            // Check for tasks/peers that have already completed.
            while let Some(Ok(res)) = handles.try_join_next() {
                println!("Task finished!");
                match res {
                    PieceDownloadResult::Success { peer, piece } => {
                        assert!(active_peers.remove(&peer.socket_addr()));
                    }
                    PieceDownloadResult::Error {
                        peer_socket_addr,
                        piece_des,
                    } => {
                        assert!(active_peers.remove(&peer_socket_addr));
                        self.piece_queue.push_back(piece_des);
                    }
                }
            }

            if active_peers.is_empty() && self.piece_queue.is_empty() {
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        tracker_handle.abort();

        Ok(())
    }
}

enum PieceDownloadResult {
    Success {
        peer: Peer<Connected>,
        piece: (PieceDescriptor, Vec<u8>),
    },
    Error {
        peer_socket_addr: SocketAddrV4,
        piece_des: PieceDescriptor,
    },
}
