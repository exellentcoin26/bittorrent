use std::{
    collections::HashSet, fs::File, net::SocketAddrV4, ops::Deref, path::Path, sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use crossbeam_deque::{Injector, Steal};
use tokio::{
    sync::{mpsc, watch, RwLock},
    task::JoinSet,
};

use crate::{
    peer::{Connected, Peer, PieceDescriptor},
    torrent::Torrent,
    tracker::{Tracker, TrackerResponse},
    util::Sha1Hash,
    util::{calculate_piece_length, PeerId},
};

const MAX_CONCURRENT_DOWNLOADS: u32 = 20;

pub struct TorrentDownloader {
    piece_queue: Arc<Injector<PieceDescriptor>>,
    tracker: Tracker,
    client_peer_id: PeerId,
}

fn generate_piece_queue(
    piece_hashes: Vec<Sha1Hash>,
    piece_length: u32,
    torrent_length: u64,
) -> Injector<PieceDescriptor> {
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

    let piece_queue = Injector::new();
    for des in piece_descriptors {
        piece_queue.push(des);
    }

    piece_queue
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

        let piece_queue = generate_piece_queue(piece_hashes, piece_length, torrent_length).into();

        // let peers = peer_socket_addresses.into_iter().map(Peer::from_socket);
        // let mut connected_peers = Vec::with_capacity(peers.size_hint().0);
        // for peer in peers {
        //     let peer = peer
        //         .handshake(torrent.info_hash, client_peer_id)
        //         .await
        //         .context("performing peer handshake")?;
        //
        //     connected_peers.push(peer);
        // }

        Ok(Self {
            piece_queue,
            tracker,
            client_peer_id,
            // peers: connected_peers,
        })
    }

    pub async fn download(self, _output_location: impl AsRef<Path>) -> Result<()> {
        // For every peer available to download, start a task that downloads and returns the piece.
        // If a new peer becomes available, immediatly pick up a new task.
        // A peer should be donated to a task and returned when done downloading that piece.
        //
        // # Idea
        //
        // Tracker polling task that inserts peers as they become available.
        // Main loop -> Checks channel for new peers, creates tasks and checks if tasks have been
        // completed.

        // let peer_amount = self.peers.len();
        // let mut waiting_task_amount = Arc::new(RwLock::new(0));

        let mut handles = JoinSet::new();

        let (tracker_tx, mut tracker_rx) = watch::channel(None);
        let (deactivate_peer_tx, mut deactivate_peer_rx) =
            mpsc::channel(MAX_CONCURRENT_DOWNLOADS as usize);
        let mut active_peers: HashSet<SocketAddrV4> = HashSet::new();

        let tracker_handle = tokio::spawn(async move {
            let tracker = self.tracker;
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
        });

        'main: loop {
            println!("Doing next iteration");

            // Update active peer set.
            while let Some(peer) = match deactivate_peer_rx.try_recv() {
                Ok(peer) => {
                    println!("Deactivating peer");
                    Some(peer)
                }
                Err(mpsc::error::TryRecvError::Empty) => None,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    panic!("tracker task should not be dropped before main event loop")
                }
            } {
                assert!(active_peers.remove(&peer));
            }

            // Wait for new peers to be discoverred.
            let new_peers = {
                println!("Trying to check new value");
                let ps = tracker_rx.borrow_and_update();

                let Some(ref ps) = *ps else {
                    // Avoid a deadlock by dropping the value early.
                    drop(ps);
                    println!("No peers present");
                    // Allow some breathing room.
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                };

                println!("Received some peers");
                // dbg!(&ps);
                ps.clone()
            };

            let mut new_active_peers = HashSet::new();
            // Start a task for every peer that is inactive.
            for peer in new_peers
                .into_socket_addrs()
                .into_iter()
                .filter(|p| !active_peers.contains(p))
            {
                let piece_des = match self.piece_queue.steal() {
                    Steal::Success(p) => p,
                    Steal::Retry => continue,
                    Steal::Empty => {
                        // TODO: Check if all peers have finished their tasks and only then stop
                        // the main loop.
                        break 'main;
                    }
                };

                println!("Taking piece descriptor from the queue");

                let deactivate_peer_tx = deactivate_peer_tx.clone();
                handles.spawn(async move {
                    // Download the piece.

                    deactivate_peer_tx.send(peer).await.expect(
                        "deactive peer channel should not be closed before closing peer tasks",
                    );
                });

                new_active_peers.insert(peer);
            }

            active_peers.extend(new_active_peers);

            // Check for tasks/peers that have already completed.
            while let Some(Ok(_)) = handles.try_join_next() {
                println!("Task finished!");
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }
}
