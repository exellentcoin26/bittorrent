use std::{fs::File, net::SocketAddrV4, path::Path};

use anyhow::{Context, Result};
use crossbeam_deque::Injector;
use tokio::task::JoinSet;

use crate::{
    peer::{Connected, Peer, PieceDescriptor},
    torrent::Torrent,
    util::Sha1Hash,
    util::{calculate_piece_length, PeerId},
};

pub struct TorrentDownloader {
    piece_queue: Injector<PieceDescriptor>,
    peers: Vec<Peer<Connected>>,
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
        peer_socket_addresses: impl IntoIterator<Item = SocketAddrV4>,
        client_peer_id: PeerId,
    ) -> Result<Self> {
        let torrent_length = torrent.info.length;
        let piece_length = torrent.info.piece_length;
        let piece_hashes = torrent.info.pieces;

        let piece_queue = generate_piece_queue(piece_hashes, piece_length, torrent_length);

        let peers = peer_socket_addresses.into_iter().map(Peer::from_socket);
        let mut connected_peers = Vec::with_capacity(peers.size_hint().0);
        for peer in peers {
            let peer = peer
                .handshake(torrent.info_hash, client_peer_id)
                .await
                .context("performing peer handshake")?;

            connected_peers.push(peer);
        }

        Ok(Self {
            piece_queue,
            peers: connected_peers,
        })
    }

    pub async fn download(self, _output_location: impl AsRef<Path>) -> Result<()> {
        let mut handles = JoinSet::new();

        for _peer in self.peers {
            handles.spawn(async {});
        }

        while let Some(_piece) = handles.join_next().await {
            // Write piece to file.
        }

        Ok(())
    }
}
