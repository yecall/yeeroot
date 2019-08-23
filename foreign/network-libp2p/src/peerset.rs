// Copyright (C) 2019 Yee Foundation.
//
// This file is part of YeeChain.
//
// YeeChain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// YeeChain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with YeeChain.  If not, see <https://www.gnu.org/licenses/>.

pub use substrate_peerset::Message;
pub use substrate_peerset::IncomingIndex;
use std::collections::{HashMap, VecDeque};
use substrate_peerset::{Peerset, PeersetHandle, PeersetConfig};
use libp2p::PeerId;
use std::sync::Arc;
use futures::{prelude::*, sync::mpsc, try_ready};
use futures::stream::Fuse;
use log::{info, debug};
use parking_lot::RwLock;

pub struct ForeignPeerset {
    peersets: HashMap<u16, Peerset>,
    handle: ForeignPeersetHandle,
    config: ForeignPeersetConfig,
    router: ForeignPeerRouter,
    incoming_queue: VecDeque<(PeerId, IncomingIndex)>,
}

#[derive(Clone)]
pub struct ForeignPeersetHandle {
    handles: Arc<RwLock<HashMap<u16, PeersetHandle>>>,
    router: ForeignPeerRouter,
}

#[derive(Clone)]
pub struct ForeignPeerRouter(Arc<RwLock<HashMap<PeerId, u16>>>);

pub struct ForeignPeersetConfig {
    pub in_peers: u32,

    pub out_peers: u32,

    pub foreign_boot_nodes: HashMap<u16, Vec<PeerId>>,
}

impl ForeignPeerset {
    pub fn from_config(config: ForeignPeersetConfig) -> (ForeignPeerset, ForeignPeersetHandle) {
        let router = ForeignPeerRouter(Arc::new(RwLock::new(HashMap::new())));

        let handle = ForeignPeersetHandle {
            handles: Arc::new(RwLock::new(HashMap::new())),
            router: router.clone(),
        };

        let peerset = ForeignPeerset {
            peersets: HashMap::new(),
            handle: handle.clone(),
            config,
            router,
            incoming_queue: VecDeque::new(),
        };

        (peerset, handle)
    }

    pub fn dropped(&mut self, peer_id: PeerId) {
        if let Some(shard_num) = self.router.get_shard_num(&peer_id) {
            if let Some(peerset) = self.peersets.get_mut(&shard_num) {
                peerset.dropped(peer_id);
            }
        }
    }

    pub fn incoming(&mut self, peer_id: PeerId, index: IncomingIndex) {
        debug!(target: "sub-libp2p-foreign", "queue incoming: peer_id: {}, index: {:?}", peer_id, index);
        self.incoming_queue.push_back((peer_id, index));
    }

    pub fn discovered(&mut self, peer_id: PeerId, shard_num: u16) {
        self.touch(shard_num);

        match self.peersets.get_mut(&shard_num) {
            Some(peerset) => {
                peerset.discovered(peer_id.clone());
            }
            None => {}
        }
        self.router.0.write().insert(peer_id, shard_num);
    }

    pub fn debug_info(&self) -> serde_json::Value {
        serde_json::Value::String(format!("{:?}", self.peersets))
    }

    pub fn router(&self) -> ForeignPeerRouter {
        self.router.clone()
    }

    fn touch(&mut self, shard_num: u16) {
        if let None = self.peersets.get(&shard_num) {
            let (peerset, handle) = Peerset::from_config(PeersetConfig {
                in_peers: self.config.in_peers,
                out_peers: self.config.out_peers,
                bootnodes: self.config.foreign_boot_nodes.get(&shard_num).cloned().unwrap_or(Vec::new()),
                reserved_only: false,
                reserved_nodes: Vec::new(),
            });
            self.peersets.insert(shard_num, peerset);
            self.handle.handles.write().insert(shard_num, handle);
        }
    }
}

impl Stream for ForeignPeerset {
    type Item = Message;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {

        let mut requeue = Vec::new();
        while let Some((peer_id, index)) = self.incoming_queue.pop_front() {
            if let Some(shard_num) = self.router.get_shard_num(&peer_id) {
                if let Some(peerset) = self.peersets.get_mut(&shard_num) {
                    debug!(target: "sub-libp2p-foreign", "process incoming: peer_id: {}, index: {:?}, shard_num: {}", peer_id, index, shard_num);
                    peerset.incoming(peer_id, index);
                }
            } else {
                debug!(target: "sub-libp2p-foreign", "requeue incoming: peer_id: {}, index: {:?}", peer_id, index);
                requeue.push((peer_id, index));
            }
        }

        for (peer_id, index) in requeue{
            self.incoming_queue.push_back((peer_id, index));
        }

        for (_shard_num, peerset) in &mut self.peersets {
            match peerset.fuse().poll() {
                Ok(Async::Ready(t)) => {
                    match t {
                        Some(t) => return Ok(Async::Ready(Some(t))),
                        None => {}
                    }
                }
                _ => {}
            }
        }
        Ok(Async::NotReady)
    }
}

impl ForeignPeersetHandle {
    pub fn report_peer(&self, peer_id: PeerId, score_diff: i32) {
        if let Some(shard_num) = self.router.0.read().get(&peer_id) {
            if let Some(handle) = self.handles.read().get(shard_num) {
                handle.report_peer(peer_id, score_diff);
            }
        }
    }
}

impl ForeignPeerRouter {
    pub fn get_shard_num(&self, peer_id: &PeerId) -> Option<u16> {
        self.0.read().get(peer_id).cloned()
    }
}