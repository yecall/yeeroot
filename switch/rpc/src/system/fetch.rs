

use super::*;
use network::{self, ProtocolStatus, PeerId, PeerInfo as NetworkPeerInfo};
use network::config::Roles;
use futures::sync::mpsc;
use yee_runtime::opaque::Block;

pub struct Status {
    pub peers: usize,
    pub is_syncing: bool,
    pub is_dev: bool,
    pub peer_id: PeerId,
}

impl Default for Status {
    fn default() -> Status {
        Status {
            peer_id: PeerId::random(),
            peers: 0,
            is_syncing: false,
            is_dev: false,
        }
    }
}

impl network::SyncProvider<Block> for Status {
    fn status(&self) -> mpsc::UnboundedReceiver<ProtocolStatus<Block>> {
        let (_sink, stream) = mpsc::unbounded();
        stream
    }

    fn network_state(&self) -> network::NetworkState {
        network::NetworkState {
            peer_id: String::new(),
            listened_addresses: Default::default(),
            external_addresses: Default::default(),
            connected_peers: Default::default(),
            not_connected_peers: Default::default(),
            average_download_per_sec: 0,
            average_upload_per_sec: 0,
            peerset: serde_json::Value::Null,
        }
    }

    fn peers(&self) -> Vec<(PeerId, NetworkPeerInfo<Block>)> {
        let mut peers = vec![];
        for _peer in 0..self.peers {
            peers.push(
                (self.peer_id.clone(), NetworkPeerInfo {
                    roles: Roles::FULL,
                    protocol_version: 1,
                    best_hash: Default::default(),
                    best_number: 1
                })
            );
        }
        peers
    }

    fn is_major_syncing(&self) -> bool {
        self.is_syncing
    }
}


pub fn api<T: Into<Option<Status>>>(sync: T) -> System<Block> {
    let status = sync.into().unwrap_or_default();
    let should_have_peers = !status.is_dev;
    System::new(SystemInfo {
        impl_name: "yee-testclient".into(),
        impl_version: "0.2.0".into(),
        chain_name: "yeetestchain".into(),
        properties: Default::default(),
    }, Arc::new(status), should_have_peers)
}



