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

use substrate_network::message::{self, BlockRequest as BlockRequestMessage, Message};
use substrate_network::message::generic::{Message as GenericMessage, ConsensusMessage};
use runtime_primitives::traits::{As, Block as BlockT, Header as HeaderT, NumberFor, Zero};
use substrate_network::config::{ProtocolConfig, Roles};
use std::sync::Arc;
use crate::service::{NetworkChan, ExHashT};
use crate::NetworkMsg;
use crate::util::LruHashSet;
use crate::chain::Client;
use crate::error;
use network_libp2p::{PeerId, Severity};
use runtime_primitives::{generic::BlockId, ConsensusEngineId, Proof, traits::BlakeTwo256};
use log::{trace, debug, info};
use std::{cmp, num::NonZeroUsize, thread, time};
use std::collections::{BTreeMap, HashMap};
use substrate_network::{SyncStatus, OnDemandService};
use parking_lot::RwLock;
use merkle_light::merkle::MerkleTree;
use ansi_term::Colour;

const REQUEST_TIMEOUT_SEC: u64 = 40;
/// Interval at which we perform time based maintenance
const TICK_TIMEOUT: time::Duration = time::Duration::from_millis(1100);
/// Interval at which we propagate exstrinsics;
const PROPAGATE_TIMEOUT: time::Duration = time::Duration::from_millis(2900);
/// Interval at which we send status updates on the SyncProvider status stream.
const STATUS_INTERVAL: time::Duration = time::Duration::from_millis(5000);

/// Current protocol version.
pub(crate) const CURRENT_VERSION: u32 = 2;
/// Lowest version we support
const MIN_VERSION: u32 = 2;

// Maximum allowed entries in `BlockResponse`
const MAX_BLOCK_DATA_RESPONSE: u32 = 128;
/// When light node connects to the full node and the full node is behind light node
/// for at least `LIGHT_MAXIMAL_BLOCKS_DIFFERENCE` blocks, we consider it unuseful
/// and disconnect to free connection slot.
const LIGHT_MAXIMAL_BLOCKS_DIFFERENCE: u64 = 8192;

pub struct VProtocol<B: BlockT, H: ExHashT> {
    network_chan: NetworkChan<B>,
    config: ProtocolConfig,
    genesis_hash: B::Hash,
    context_data: ContextData<B, H>,
    handshaking_peers: HashMap<PeerId, HandshakingPeer>,
    connected_peers: Arc<RwLock<HashMap<PeerId, ConnectedPeer<B>>>>,
    protocol_context_data: Arc<RwLock<crate::protocol::ContextData<B, H>>>,
}

/// A peer from whom we have received a Status message.
#[derive(Clone)]
pub struct ConnectedPeer<B: BlockT> {
    pub peer_info: PeerInfo<B>
}

/// A peer that we are connected to
/// and from whom we have not yet received a Status message.
struct HandshakingPeer {
    timestamp: time::Instant,
}

/// Syncing status and statistics
#[derive(Clone)]
pub struct ProtocolStatus<B: BlockT> {
    /// Sync status.
    pub sync: SyncStatus<B>,
    /// Total number of connected peers
    pub num_peers: usize,
    /// Total number of active peers.
    pub num_active_peers: usize,
}

/// Peer information
#[derive(Debug)]
struct Peer<B: BlockT, H: ExHashT> {
    info: PeerInfo<B>,
    /// Current block request, if any.
    block_request: Option<(time::Instant, message::BlockRequest<B>)>,
    /// Requests we are no longer insterested in.
    obsolete_requests: HashMap<message::RequestId, time::Instant>,
    /// Holds a set of transactions known to this peer.
    known_extrinsics: LruHashSet<H>,
    /// Holds a set of blocks known to this peer.
    known_blocks: LruHashSet<B::Hash>,
    /// Request counter,
    next_request_id: message::RequestId,
}

/// Info about a peer's known state.
#[derive(Clone, Debug)]
pub struct PeerInfo<B: BlockT> {
    /// Roles
    pub roles: Roles,
    /// Protocol version
    pub protocol_version: u32,
    /// Peer best block hash
    pub best_hash: B::Hash,
    /// Peer best block number
    pub best_number: <B::Header as HeaderT>::Number,
}

/// Context for a network-specific handler.
pub trait Context<B: BlockT> {
    /// Get a reference to the client.
    fn client(&self) -> &crate::chain::Client<B>;

    /// Point out that a peer has been malign or irresponsible or appeared lazy.
    fn report_peer(&mut self, who: PeerId, reason: Severity);

    /// Get peer info.
    fn peer_info(&self, peer: &PeerId) -> Option<PeerInfo<B>>;

    /// Request a block from a peer.
    fn send_block_request(&mut self, who: PeerId, request: BlockRequestMessage<B>);

    /// Send a consensus message to a peer.
    fn send_consensus(&mut self, who: PeerId, consensus: ConsensusMessage);

    /// Send a chain-specific message to a peer.
    fn send_chain_specific(&mut self, who: PeerId, message: Vec<u8>);
}

/// Protocol context.
struct ProtocolContext<'a, B: 'a + BlockT, H: 'a + ExHashT> {
    network_chan: &'a NetworkChan<B>,
    context_data: &'a mut ContextData<B, H>,
}

impl<'a, B: BlockT + 'a, H: 'a + ExHashT> ProtocolContext<'a, B, H> {
    fn new(context_data: &'a mut ContextData<B, H>, network_chan: &'a NetworkChan<B>) -> Self {
        ProtocolContext { network_chan, context_data }
    }
}

impl<'a, B: BlockT + 'a, H: ExHashT + 'a> Context<B> for ProtocolContext<'a, B, H> {
    fn report_peer(&mut self, who: PeerId, reason: Severity) {
        self.network_chan.send(NetworkMsg::ReportPeer(who, reason))
    }

    fn peer_info(&self, who: &PeerId) -> Option<PeerInfo<B>> {
        self.context_data.peers.get(who).map(|p| p.info.clone())
    }

    fn client(&self) -> &Client<B> {
        &*self.context_data.chain
    }

    fn send_block_request(&mut self, who: PeerId, request: BlockRequestMessage<B>) {
        send_message(&mut self.context_data.peers, &self.network_chan, who,
                     GenericMessage::BlockRequest(request), self.context_data.shard_num,
        )
    }

    fn send_consensus(&mut self, who: PeerId, consensus: ConsensusMessage) {
        send_message(&mut self.context_data.peers, &self.network_chan, who,
                     GenericMessage::Consensus(consensus), self.context_data.shard_num,
        )
    }

    fn send_chain_specific(&mut self, who: PeerId, message: Vec<u8>) {
        send_message(&mut self.context_data.peers, &self.network_chan, who,
                     GenericMessage::ChainSpecific(message), self.context_data.shard_num,
        )
    }
}

/// Data necessary to create a context.
struct ContextData<B: BlockT, H: ExHashT> {
    /// All connected peers
    peers: HashMap<PeerId, Peer<B, H>>,
    pub chain: Arc<Client<B>>,
    /// self full node sharding number.
    shard_num: u16,
}

impl<B: BlockT, H: ExHashT> VProtocol<B, H> {
    pub fn new(
        network_chan: NetworkChan<B>,
        chain: Arc<Client<B>>,
        shard_num: u16,
        protocol_context_data: Arc<RwLock<crate::protocol::ContextData<B, H>>>,
    ) -> error::Result<Self> {
        let config = ProtocolConfig {
            roles: Roles::LIGHT,
        };
        let info = chain.info()?;
        let peers: Arc<RwLock<HashMap<PeerId, ConnectedPeer<B>>>> = Arc::new(Default::default());
        Ok(Self {
            network_chan,
            config,
            genesis_hash: info.chain.genesis_hash,
            context_data: ContextData {
                peers: HashMap::new(),
                chain,
                shard_num,
            },
            handshaking_peers: HashMap::new(),
            connected_peers: peers,
            protocol_context_data,
        })
    }

    /// Called when a new peer is connected
    pub fn on_peer_connected(&mut self, who: PeerId, debug_info: String) {
        trace!(target: "sync-foreign", "VProtocol: Connecting {}: {}", who, debug_info);
        self.handshaking_peers.insert(who.clone(), HandshakingPeer { timestamp: time::Instant::now() });
        self.send_status(who);
    }

    /// Called by peer when it is disconnecting
    pub fn on_peer_disconnected(&mut self, peer: PeerId, debug_info: String) {
        trace!(target: "sync-foreign", "VProtocol: Disconnecting {}: {}", peer, debug_info);
        // lock all the the peer lists so that add/remove peer events are in order
        let removed = {
            self.handshaking_peers.remove(&peer);
            self.connected_peers.write().remove(&peer);
            self.context_data.peers.remove(&peer).is_some()
        };
    }

    pub fn on_vmessage(&mut self, who: PeerId, vmessage: Message<B>) {
        match vmessage {
            GenericMessage::Status(s) => self.on_status_message(who, s),
            GenericMessage::BlockRequest(r) => self.on_block_request(who, r),
            other => {
                debug!(target: "sync-foreign", "VProtocol: other: {:?}", other);
            }
        }
    }

    pub fn on_block_imported(&mut self, hash: B::Hash, header: &B::Header) {
        // send out block announcements
        let message = GenericMessage::BlockAnnounce(message::BlockAnnounce { header: header.clone() });

        for (who, ref mut peer) in self.context_data.peers.iter_mut() {
            if peer.known_blocks.insert(hash.clone()) {
                trace!(target: "sync-foreign", "VProtocol: Announcing block {:?} to {}", hash, who);
                let message = crate::generic_message::Message::VMessage(self.context_data.shard_num, message.clone());
                self.network_chan.send(NetworkMsg::Outgoing(who.clone(), message));
            }
        }
    }

    /// Called by peer to report status
    fn on_status_message(&mut self, who: PeerId, status: message::Status<B>) {
        trace!(target: "sync-foreign", "VProtocol: New peer {} {:?}", who, status);
        {
            if self.context_data.peers.contains_key(&who) {
                debug!("VProtocol: Unexpected status packet from {}", who);
                return;
            }
            if status.genesis_hash != self.genesis_hash {
                let reason = format!(
                    "Peer is on different chain (our genesis: {} theirs: {})",
                    self.genesis_hash, status.genesis_hash
                );
                self.network_chan.send(NetworkMsg::ReportPeer(
                    who,
                    Severity::Bad(reason),
                ));
                return;
            }
            if status.version < MIN_VERSION && CURRENT_VERSION < status.min_supported_version {
                let reason = format!("Peer using unsupported protocol version {}", status.version);
                self.network_chan.send(NetworkMsg::ReportPeer(
                    who,
                    Severity::Bad(reason),
                ));
                return;
            }
            if self.config.roles & Roles::LIGHT == Roles::LIGHT {
                let self_best_block = self
                    .context_data
                    .chain
                    .info()
                    .ok()
                    .and_then(|info| info.best_queued_number)
                    .unwrap_or_else(|| Zero::zero());
                let blocks_difference = self_best_block
                    .as_()
                    .checked_sub(status.best_number.as_())
                    .unwrap_or(0);
                if blocks_difference > LIGHT_MAXIMAL_BLOCKS_DIFFERENCE {
                    self.network_chan.send(NetworkMsg::ReportPeer(
                        who,
                        Severity::Useless(
                            "Peer is far behind us and will unable to serve light requests"
                                .to_string(),
                        ),
                    ));
                    return;
                }
            }

            let cache_limit = NonZeroUsize::new(1_000_000).expect("1_000_000 > 0; qed");

            let info = match self.handshaking_peers.remove(&who) {
                Some(_handshaking) => {
                    let peer_info = PeerInfo {
                        protocol_version: status.version,
                        roles: status.roles,
                        best_hash: status.best_hash,
                        best_number: status.best_number,
                    };
                    self.connected_peers
                        .write()
                        .insert(who.clone(), ConnectedPeer { peer_info: peer_info.clone() });
                    peer_info
                }
                None => {
                    debug!(target: "sync-foreign", "VProtocol: Received status from previously unconnected node {}", who);
                    return;
                }
            };

            let peer = Peer {
                info,
                block_request: None,
                known_extrinsics: LruHashSet::new(cache_limit),
                known_blocks: LruHashSet::new(cache_limit),
                next_request_id: 0,
                obsolete_requests: HashMap::new(),
            };
            self.context_data.peers.insert(who.clone(), peer);

            debug!(target: "sync-foreign", "VProtocol: Connected {}", who);
        }
    }

    fn on_block_request(&mut self, peer: PeerId, request: message::BlockRequest<B>) {
        trace!(target: "sync-foreign", "VProtocol: BlockRequest {} from {}: from {:?} to {:?} max {:?}",
               request.id,
               peer,
               request.from,
               request.to,
               request.max);
        let mut blocks = Vec::new();
        let mut id = match request.from {
            message::FromBlock::Hash(h) => BlockId::Hash(h),
            message::FromBlock::Number(n) => BlockId::Number(n),
        };
        let max = cmp::min(request.max.unwrap_or(u32::max_value()), MAX_BLOCK_DATA_RESPONSE) as usize;
        let get_header = request.fields.contains(message::BlockAttributes::HEADER);
        // let get_body = request.fields.contains(message::BlockAttributes::BODY);
//        let get_justification = request
//            .fields
//            .contains(message::BlockAttributes::JUSTIFICATION);
        let get_proof = request.fields.contains(message::BlockAttributes::PROOF);
        while let Some(header) = self.context_data.chain.header(&id).unwrap_or(None) {
            if blocks.len() >= max {
                break;
            }
            let number = header.number().clone();
            let hash = header.hash();
            let parent_hash = header.parent_hash().clone();
//            let justification = if get_justification {
//                self.context_data.chain.justification(&BlockId::Hash(hash)).unwrap_or(None)
//            } else {
//                None
//            };
            let proof = if get_proof {
                self.protocol_context_data.read().peers.get(&peer).map(|p| {
                    self.get_proof_by_shard_num(hash, p.info.shard_num)
                }).unwrap_or(None)
            } else {
                None
            };
            if proof.is_some() {
                debug!("receive {}: number:{}, proof.len():{}", Colour::White.bold().paint("Proof"), number, proof.clone().unwrap().len());
            } else {
                info!("Sync Block proof. {}. number:{}", Colour::White.bold().paint("No Proof"), number);
            }
            let block_data = message::generic::BlockData {
                hash,
                header: if get_header { Some(header) } else { None },
                body:  None,

                receipt: None,
                message_queue: None,
                justification: None,
                proof,
            };
            blocks.push(block_data);
            match request.direction {
                message::Direction::Ascending => id = BlockId::Number(number + As::sa(1)),
                message::Direction::Descending => {
                    if number == As::sa(0) {
                        break;
                    }
                    id = BlockId::Hash(parent_hash)
                }
            }
        }
        let response = message::generic::BlockResponse {
            id: request.id,
            blocks,
        };
        trace!(target: "sync-foreign", "VProtocol: Sending BlockResponse with {} blocks", response.blocks.len());
        self.send_message(peer, GenericMessage::BlockResponse(response))
    }

    /// Get proof by shard num.
    fn get_proof_by_shard_num(&self, hash: B::Hash, shard_num: u16) -> Option<Proof> {
        let id = BlockId::Hash(hash);
        let total_proof = self.context_data.chain.proof(&id).unwrap_or(None);
        if let Some(proof) = total_proof {
            let bytes = proof.as_slice();
            let tree = yee_merkle::MultiLayerProof::from_bytes(bytes);
            if let Ok(ml) = tree {
                if let Some(pf) = ml.gen_proof(shard_num) {
                    return Some(pf.into_bytes());
                }
            }
        }
        None
    }

    /// Send Status message
    fn send_status(&mut self, who: PeerId) {
        if let Ok(info) = self.context_data.chain.info() {
            let status = message::generic::Status {
                version: CURRENT_VERSION,
                min_supported_version: MIN_VERSION,
                genesis_hash: info.chain.genesis_hash,
                roles: self.config.roles.into(),
                best_number: info.chain.best_number,
                best_hash: info.chain.best_hash,
                chain_status: Vec::new(),
            };
            self.send_message(who, GenericMessage::Status(status))
        }
    }

    fn send_message(&mut self, who: PeerId, message: Message<B>) {
        send_message::<B, H>(
            &mut self.context_data.peers,
            &self.network_chan,
            who,
            message,
            self.context_data.shard_num,
        );
    }
}

fn send_message<B: BlockT, H: ExHashT>(
    peers: &mut HashMap<PeerId, Peer<B, H>>,
    network_chan: &NetworkChan<B>,
    who: PeerId,
    mut message: Message<B>,
    shard_num: u16,
) {
    if let GenericMessage::BlockRequest(ref mut r) = message {
        if let Some(ref mut peer) = peers.get_mut(&who) {
            r.id = peer.next_request_id;
            peer.next_request_id = peer.next_request_id + 1;
            if let Some((timestamp, request)) = peer.block_request.take() {
                trace!(target: "sync-foreign", "VProtocol: Request {} for {} is now obsolete.", request.id, who);
                peer.obsolete_requests.insert(request.id, timestamp);
            }
            peer.block_request = Some((time::Instant::now(), r.clone()));
        }
    }
    let message = crate::generic_message::Message::VMessage(shard_num, message);
    network_chan.send(NetworkMsg::Outgoing(who, message));
}
