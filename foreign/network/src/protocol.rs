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
use crate::message::generic::{Message as GenericMessage};
use crate::service::{NetworkChan, NetworkMsg, ExHashT};
use parking_lot::RwLock;
use rustc_hex::ToHex;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::{cmp, num::NonZeroUsize, thread, time};
use log::{trace, debug, warn};
use crate::{error, util::LruHashSet};

/// Interval at which we propagate exstrinsics;
const PROPAGATE_TIMEOUT: time::Duration = time::Duration::from_millis(2900);

/// Current protocol version.
pub(crate) const CURRENT_VERSION: u32 = 2;
/// Lowest version we support
const MIN_VERSION: u32 = 2;

// Lock must always be taken in order declared here.
pub struct Protocol<B: BlockT> {
	network_chan: NetworkChan<B>,
	port: Receiver<ProtocolMsg<B>>,
	from_network_port: Receiver<FromNetworkMsg<B>>,
}

/// Messages sent to Protocol from elsewhere inside the system.
pub enum ProtocolMsg<B: BlockT> {
	/// Tell protocol to propagate extrinsics.
	PropagateExtrinsics,
	Stop,
	/// Synchronization request.
	#[cfg(any(test, feature = "test-helpers"))]
	Synchronize,
	Phantom(B),
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

enum Incoming<B: BlockT> {
	FromNetwork(FromNetworkMsg<B>),
	FromClient(ProtocolMsg<B>)
}

impl<B: BlockT> Protocol<B> {
	/// Create a new instance.
	pub fn new(
		is_offline: Arc<AtomicBool>,
		is_major_syncing: Arc<AtomicBool>,
		network_chan: NetworkChan<B>,
	) -> error::Result<(Sender<ProtocolMsg<B>>, Sender<FromNetworkMsg<B>>)> {
		let (protocol_sender, port) = channel::unbounded();
		let (from_network_sender, from_network_port) = channel::bounded(4);
		let _ = thread::Builder::new()
			.name("Protocol".into())
			.spawn(move || {
				let mut protocol = Protocol {
					network_chan,
					from_network_port,
					port,
				};
				let propagate_timeout = channel::tick(PROPAGATE_TIMEOUT);
				while protocol.run(&propagate_timeout) {
					// Running until all senders have been dropped...
				}
			})
			.expect("Protocol thread spawning failed");
		Ok((protocol_sender, from_network_sender))
	}

	fn run(
		&mut self,
		propagate_timeout: &Receiver<time::Instant>,
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
			recv(propagate_timeout) -> _ => {
				Incoming::FromClient(ProtocolMsg::PropagateExtrinsics)
			},
		};
		self.handle_msg(msg)
	}

	fn handle_msg(&mut self, msg: Incoming<B>) -> bool {
		match msg {
			Incoming::FromNetwork(msg) => self.handle_network_msg(msg),
			Incoming::FromClient(msg) => self.handle_client_msg(msg),
		}
	}

	fn handle_client_msg(&mut self, msg: ProtocolMsg<B>) -> bool {
		match msg {
			ProtocolMsg::PropagateExtrinsics => self.propagate_extrinsics(),
			ProtocolMsg::Stop => {
				self.stop();
				return false;
			},
			#[cfg(any(test, feature = "test-helpers"))]
			ProtocolMsg::Synchronize => self.network_chan.send(NetworkMsg::Synchronized),
			ProtocolMsg::Phantom(..) => {},
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
			GenericMessage::Transactions(m) => self.on_extrinsics(who, m),
		}
	}

	/// Called when a new peer is connected
	fn on_peer_connected(&mut self, who: PeerId, debug_info: String) {
		trace!(target: "sync-foreign", "Connecting {}: {}", who, debug_info);
	}

	/// Called by peer when it is disconnecting
	fn on_peer_disconnected(&mut self, peer: PeerId, debug_info: String) {
		trace!(target: "sync-foreign", "Disconnecting {}: {}", peer, debug_info);
	}

	/// Called as a back-pressure mechanism if the networking detects that the peer cannot process
	/// our messaging rate fast enough.
	pub fn on_clogged_peer(&self, who: PeerId, _msg: Option<Message<B>>) {
		trace!(target: "sync-foreign", "Clogged peer {}", who);
	}

	/// Called when peer sends us new extrinsics
	fn on_extrinsics(&mut self, who: PeerId, extrinsics: message::Transactions<B::Extrinsic>) {
		trace!(target: "sync-foreign", "Received {} extrinsics from {}", extrinsics.len(), who);
	}

	/// Called when we propagate ready extrinsics to peers.
	fn propagate_extrinsics(&mut self) {
		debug!(target: "sync-foreign", "Propagating extrinsics");
	}

	fn stop(&mut self) {

	}
}
