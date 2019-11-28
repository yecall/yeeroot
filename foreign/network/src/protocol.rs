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

use crossbeam_channel::{self as channel, Receiver, Sender, select};
use futures::sync::mpsc;
use parking_lot::Mutex;
use network_libp2p::{PeerId, Severity};
use primitives::storage::StorageKey;
use runtime_primitives::{generic::BlockId, ConsensusEngineId};
use runtime_primitives::traits::{As, Block as BlockT, Header as HeaderT, NumberFor, Zero};
use crate::message::{self, Message};
use crate::message::generic::{Message as GenericMessage, OutMessage};
use crate::service::{NetworkChan, NetworkMsg, ExHashT};
use parking_lot::RwLock;
use rustc_hex::ToHex;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::{cmp, num::NonZeroUsize, thread, time};
use log::{trace, debug, warn};
use crate::{error, util::LruHashSet};
use crate::chain::Client;
use serde::export::PhantomData;
use crate::config::{NetworkConfiguration, ProtocolConfig};
use crate::vprotocol::VProtocol;

/// Current protocol version.
pub(crate) const CURRENT_VERSION: u32 = 2;
/// Lowest version we support
const MIN_VERSION: u32 = 2;

// Lock must always be taken in order declared here.
pub struct Protocol<B: BlockT, H: ExHashT> {
	out_message_sinks: Arc<Mutex<Vec<mpsc::UnboundedSender<OutMessage<B>>>>>,
	network_chan: NetworkChan<B>,
	port: Receiver<ProtocolMsg<B, H>>,
	from_network_port: Receiver<FromNetworkMsg<B>>,
	config: ProtocolConfig,
	genesis_hash: B::Hash,
	/// record for remote full node
	context_data: Arc<RwLock<ContextData<B, H>>>,
	// Connected peers pending Status message.
	handshaking_peers: HashMap<PeerId, HandshakingPeer>,
	vprotocol: VProtocol<B, H>,
}

/// A peer that we are connected to
/// and from whom we have not yet received a Status message.
struct HandshakingPeer {
	timestamp: time::Instant,
}

/// Peer information
#[derive(Debug)]
pub struct Peer<B: BlockT, H: ExHashT> {
	pub info: PeerInfo<B>,
	/// Holds a set of transactions known to this peer.
	known_extrinsics: LruHashSet<H>,
}

/// Info about a peer's known state.
#[derive(Clone, Debug)]
pub struct PeerInfo<B: BlockT> {
	/// Protocol version
	pub protocol_version: u32,
	/// Peer best block hash
	pub best_hash: B::Hash,
	/// Peer best block number
	pub best_number: <B::Header as HeaderT>::Number,
	/// Shard num
	pub shard_num: u16,
}

/// Data necessary to create a context.
pub struct ContextData<B: BlockT, H: ExHashT> {
	// All connected peers
	pub peers: HashMap<PeerId, Peer<B, H>>,
	pub chain: Arc<Client<B>>,
}

/// Messages sent to Protocol from elsewhere inside the system.
pub enum ProtocolMsg<B: BlockT, H: ExHashT> {
	/// Relay Extrinsics
	RelayExtrinsics(u16, Vec<(H, B::Extrinsic)>),
	/// A block has been imported (sent by the client).
	BlockImported(B::Hash, B::Header),
	Stop,
	/// Synchronization request.
	#[cfg(any(test, feature = "test-helpers"))]
	Synchronize,
}

/// Messages sent to Protocol from Network-libp2p.
pub enum FromNetworkMsg<B: BlockT> {
	/// A peer connected, with debug info.
	PeerConnected(PeerId, String),
	/// A peer disconnected, with debug info.
	PeerDisconnected(PeerId, String),
	/// A custom message from another peer.
	CustomMessage(PeerId, Message<B>),
	/// Let protocol know a peer is currenlty clogged.
	PeerClogged(PeerId, Option<Message<B>>),
	/// Synchronization request.
	#[cfg(any(test, feature = "test-helpers"))]
	Synchronize,
}

enum Incoming<B: BlockT, H: ExHashT> {
	FromNetwork(FromNetworkMsg<B>),
	FromClient(ProtocolMsg<B, H>)
}

impl<B: BlockT, H: ExHashT> Protocol<B, H> {
	/// Create a new instance.
	pub fn new(
		out_message_sinks: Arc<Mutex<Vec<mpsc::UnboundedSender<OutMessage<B>>>>>,
		is_offline: Arc<AtomicBool>,
		is_major_syncing: Arc<AtomicBool>,
		network_chan: NetworkChan<B>,
		config: ProtocolConfig,
		chain: Arc<Client<B>>,
	) -> error::Result<(Sender<ProtocolMsg<B, H>>, Sender<FromNetworkMsg<B>>)> {
		let (protocol_sender, port) = channel::unbounded();
		let (from_network_sender, from_network_port) = channel::bounded(4);
		let info = chain.info()?;

		let context = Arc::new(RwLock::new(
			ContextData {
				peers: HashMap::<PeerId, Peer<B, H>>::new(),
				chain: chain.clone(),
			}));
		let vprotocol = VProtocol::new(
			network_chan.clone(),
			chain.clone(),
			config.shard_num,
			context.clone(),
		)?;

		let _ = thread::Builder::new()
			.name("Protocol".into())
			.spawn(move || {
				let mut protocol = Protocol {
					out_message_sinks,
					network_chan,
					from_network_port,
					config,
					port,
					genesis_hash: info.chain.genesis_hash,
					context_data: context,
					handshaking_peers: HashMap::new(),
					vprotocol,
				};
				while protocol.run() {
					// Running until all senders have been dropped...
				}
			})
			.expect("Protocol thread spawning failed");
		Ok((protocol_sender, from_network_sender))
	}

	fn run(
		&mut self
	) -> bool {
		let msg = select! {
			recv(self.port) -> event => {
				match event {
					Ok(msg) => Incoming::FromClient(msg),
					// Our sender has been dropped, quit.
					Err(_) => {
						Incoming::FromClient(ProtocolMsg::Stop)
					},
				}
			},
			recv(self.from_network_port) -> event => {
				match event {
					Ok(msg) => Incoming::FromNetwork(msg),
					// Our sender has been dropped, quit.
					Err(_) => {
						Incoming::FromClient(ProtocolMsg::Stop)
					},
				}
			},
		};
		self.handle_msg(msg)
	}

	fn handle_msg(&mut self, msg: Incoming<B, H>) -> bool {
		match msg {
			Incoming::FromNetwork(msg) => self.handle_network_msg(msg),
			Incoming::FromClient(msg) => self.handle_client_msg(msg),
		}
	}

	fn handle_client_msg(&mut self, msg: ProtocolMsg<B, H>) -> bool {
		match msg {
			ProtocolMsg::RelayExtrinsics(shard_num, extrinsics) =>
				self.on_relay_extrinsics(shard_num, extrinsics),
			ProtocolMsg::BlockImported(hash, header) =>
				self.on_block_imported(hash, &header),
			ProtocolMsg::Stop => {
				self.stop();
				return false;
			},
			#[cfg(any(test, feature = "test-helpers"))]
			ProtocolMsg::Synchronize => self.network_chan.send(NetworkMsg::Synchronized),
		}
		true
	}

	fn handle_network_msg(&mut self, msg: FromNetworkMsg<B>) -> bool {
		match msg {
			FromNetworkMsg::PeerDisconnected(who, debug_info) => self.on_peer_disconnected(who, debug_info),
			FromNetworkMsg::PeerConnected(who, debug_info) => self.on_peer_connected(who, debug_info),
			FromNetworkMsg::PeerClogged(who, message) => self.on_clogged_peer(who, message),
			FromNetworkMsg::CustomMessage(who, message) => {
				self.on_custom_message(who, message)
			},
			#[cfg(any(test, feature = "test-helpers"))]
			FromNetworkMsg::Synchronize => self.network_chan.send(NetworkMsg::Synchronized),
		}
		true
	}

	fn on_custom_message(&mut self, who: PeerId, message: Message<B>) {
		match message {
			GenericMessage::Status(s) => self.on_status_message(who, s),
			GenericMessage::RelayExtrinsics(m) => self.on_relay_extrinsics_message(who, m),
			GenericMessage::VMessage(shard_num, vmessage) => {
				if shard_num == self.config.shard_num {
					self.vprotocol.on_vmessage(who, vmessage)
				}
			},
		}
	}

	/// Called when a new peer is connected
	fn on_peer_connected(&mut self, who: PeerId, debug_info: String) {
		trace!(target: "sync-foreign", "Connecting {}: {}", who, debug_info);
		self.handshaking_peers.insert(who.clone(), HandshakingPeer { timestamp: time::Instant::now() });
		self.send_status(who.clone());

		self.vprotocol.on_peer_connected(who, debug_info);
	}

	/// Called by peer when it is disconnecting
	fn on_peer_disconnected(&mut self, peer: PeerId, debug_info: String) {
		trace!(target: "sync-foreign", "Disconnecting {}: {}", peer, debug_info);
		// lock all the the peer lists so that add/remove peer events are in order
		let removed = {
			self.handshaking_peers.remove(&peer);
			self.context_data.write().peers.remove(&peer).is_some()
		};

		self.vprotocol.on_peer_disconnected(peer, debug_info);
	}

	/// Called as a back-pressure mechanism if the networking detects that the peer cannot process
	/// our messaging rate fast enough.
	pub fn on_clogged_peer(&self, who: PeerId, _msg: Option<Message<B>>) {
		trace!(target: "sync-foreign", "Clogged peer {}", who);
		if let Some(peer) = self.context_data.read().peers.get(&who) {
			debug!(target: "sync-foreign", "Clogged peer {} (protocol_version: {:?}; \
				known_extrinsics: {:?}; best_hash: {:?}; best_number: {:?})",
				   who, peer.info.protocol_version, peer.known_extrinsics,
				   peer.info.best_hash, peer.info.best_number);
		} else {
			debug!(target: "sync", "Peer clogged before being properly connected");
		}
	}

	/// Called by peer to report status
	fn on_status_message(&mut self, who: PeerId, status: message::Status<B>) {
		trace!(target: "sync-foreign", "New peer {} {:?}", who, status);
		{
			if self.context_data.read().peers.contains_key(&who) {
				debug!("Unexpected status packet from {}", who);
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

			let cache_limit = NonZeroUsize::new(1_000_000).expect("1_000_000 > 0; qed");

			let info = match self.handshaking_peers.remove(&who) {
				Some(_handshaking) => {
					let peer_info = PeerInfo {
						protocol_version: status.version,
						best_hash: status.best_hash,
						best_number: status.best_number,
						shard_num: status.shard_num,
					};
					peer_info
				},
				None => {
					debug!(target: "sync-foreign", "Received status from previously unconnected node {}", who);
					return;
				},
			};

			let peer = Peer {
				info,
				known_extrinsics: LruHashSet::new(cache_limit),
			};
			self.context_data.write().peers.insert(who.clone(), peer);

			debug!(target: "sync-foreign", "Connected {}", who);
		}
	}

	/// Called when peer sends us new extrinsics
	fn on_relay_extrinsics_message(&mut self, who: PeerId, extrinsics: Vec<B::Extrinsic>) {
		trace!(target: "sync-foreign", "Received {} extrinsics from {}", extrinsics.len(), who);

		let message = OutMessage::RelayExtrinsics(extrinsics);
		self.out_message_sinks.lock().retain(|sink| sink.unbounded_send(message.clone()).is_ok());
	}

	fn on_relay_extrinsics(&mut self, shard_num: u16, extrinsics: Vec<(H, B::Extrinsic)>){
		debug!(target: "sync-foreign", "Relay extrinsics");

        for (who, peer) in self.context_data.write().peers.iter_mut().filter(|(_, peer)| peer.info.shard_num == shard_num) {
            let (hashes, to_send): (Vec<_>, Vec<_>) = extrinsics
                .iter()
                .filter(|&(ref hash, _)| peer.known_extrinsics.insert(hash.clone()))
                .cloned()
                .unzip();

            if !to_send.is_empty() {
                trace!(target: "sync-foreign", "Sending {} transactions to {}", to_send.len(), who);
                self.network_chan.send(NetworkMsg::Outgoing(who.clone(), GenericMessage::RelayExtrinsics(to_send)));
            }
        }
	}

	fn on_block_imported(&mut self, hash: B::Hash, header: &B::Header) {
		self.vprotocol.on_block_imported(hash, header);
	}

	/// Send Status message
	fn send_status(&mut self, who: PeerId) {
        let info = self.context_data.read().chain.info();
		if let Ok(info) = info {
			let status = message::generic::Status {
				version: CURRENT_VERSION,
				min_supported_version: MIN_VERSION,
				genesis_hash: info.chain.genesis_hash,
				best_number: info.chain.best_number,
				best_hash: info.chain.best_hash,
				chain_status: Vec::new(),
				shard_num: self.config.shard_num,
			};
			trace!(target: "sync-foreign", "Sending status to {}: gh: {} shard_num: {}", who, info.chain.genesis_hash, self.config.shard_num);
			self.send_message(who, GenericMessage::Status(status))
		}
	}

	fn stop(&mut self) {

	}

	fn send_message(&mut self, who: PeerId, message: Message<B>) {
		send_message::<B, H>(
			&self.network_chan,
			who,
			message,
		);
	}
}

fn send_message<B: BlockT, H: ExHashT>(
	network_chan: &NetworkChan<B>,
	who: PeerId,
	message: Message<B>,
) {
	network_chan.send(NetworkMsg::Outgoing(who, message));
}
