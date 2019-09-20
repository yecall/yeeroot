// Copyright 2017-2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use log::{trace};
use futures::{sync::oneshot, sync::mpsc};
use parking_lot::{Mutex, RwLock};
use consensus::import_queue::{ImportQueue, Link};
use crate::consensus_gossip::ConsensusGossip;
use crate::protocol::{Context, FromNetworkMsg, Protocol, ConnectedPeer, ProtocolMsg, ProtocolStatus};
use crate::config::Params;
use crossbeam_channel::{self as channel, Receiver, Sender, TryRecvError};
use crate::error::Error;
use runtime_primitives::{traits::{Block as BlockT, NumberFor}, ConsensusEngineId};
use crate::specialization::NetworkSpecialization;
use crate::IdentifySpecialization;

use tokio::prelude::task::AtomicTask;
use std::marker::PhantomData;

use substrate_network::{ExHashT, Severity};

/// Type that represents fetch completion future.
pub type FetchFuture = oneshot::Receiver<Vec<u8>>;

use substrate_network::service::{NetworkMsg, NetworkChan, NetworkPort, network_channel, PeerId};

/*
/// Sync status
pub trait SyncProvider<B: BlockT>: Send + Sync {
	/// Get a stream of sync statuses.
	fn status(&self) -> mpsc::UnboundedReceiver<ProtocolStatus<B>>;
	/// Get network state.
	fn network_state(&self) -> NetworkState;
	/// Get currently connected peers
	fn peers(&self) -> Vec<(PeerId, PeerInfo<B>)>;
	/// Are we in the process of downloading the chain?
	fn is_major_syncing(&self) -> bool;
}
*/

/*
/// Minimum Requirements for a Hash within Networking
pub trait ExHashT:
	::std::hash::Hash + Eq + ::std::fmt::Debug + Clone + Send + Sync + 'static
{
}
impl<T> ExHashT for T where
	T: ::std::hash::Hash + Eq + ::std::fmt::Debug + Clone + Send + Sync + 'static
{
}
*/

/*
/// Transaction pool interface
pub trait TransactionPool<H: ExHashT, B: BlockT>: Send + Sync {
	/// Get transactions from the pool that are ready to be propagated.
	fn transactions(&self) -> Vec<(H, B::Extrinsic)>;
	/// Import a transaction into the pool.
	fn import(&self, transaction: &B::Extrinsic) -> Option<H>;
	/// Notify the pool about transactions broadcast.
	fn on_broadcasted(&self, propagations: HashMap<H, Vec<String>>);
}
*/

/*
/// A link implementation that connects to the network.
#[derive(Clone)]
pub struct NetworkLink<B: BlockT, S: NetworkSpecialization<B>> {
	/// The protocol sender
	pub(crate) protocol_sender: Sender<ProtocolMsg<B, S>>,
	/// The network sender
	pub(crate) network_sender: NetworkChan<B>,
}

impl<B: BlockT, S: NetworkSpecialization<B>> Link<B> for NetworkLink<B, S> {
	fn block_imported(&self, hash: &B::Hash, number: NumberFor<B>) {
		let _ = self.protocol_sender.send(ProtocolMsg::BlockImportedSync(hash.clone(), number));
	}

	fn blocks_processed(&self, processed_blocks: Vec<B::Hash>, has_error: bool) {
		let _ = self.protocol_sender.send(ProtocolMsg::BlocksProcessed(processed_blocks, has_error));
	}

	fn justification_imported(&self, who: PeerId, hash: &B::Hash, number: NumberFor<B>, success: bool) {
		let _ = self.protocol_sender.send(ProtocolMsg::JustificationImportResult(hash.clone(), number, success));
		if !success {
			let reason = Severity::Bad(format!("Invalid justification provided for #{}", hash).to_string());
			let _ = self.network_sender.send(NetworkMsg::ReportPeer(who, reason));
		}
	}

	fn clear_justification_requests(&self) {
		let _ = self.protocol_sender.send(ProtocolMsg::ClearJustificationRequests);
	}

	fn request_justification(&self, hash: &B::Hash, number: NumberFor<B>) {
		let _ = self.protocol_sender.send(ProtocolMsg::RequestJustification(hash.clone(), number));
	}

	fn useless_peer(&self, who: PeerId, reason: &str) {
		trace!(target:"sync", "Useless peer {}, {}", who, reason);
		self.network_sender.send(NetworkMsg::ReportPeer(who, Severity::Useless(reason.to_string())));
	}

	fn note_useless_and_restart_sync(&self, who: PeerId, reason: &str) {
		trace!(target:"sync", "Bad peer {}, {}", who, reason);
		// is this actually malign or just useless?
		self.network_sender.send(NetworkMsg::ReportPeer(who, Severity::Useless(reason.to_string())));
		let _ = self.protocol_sender.send(ProtocolMsg::RestartSync);
	}

	fn restart(&self) {
		let _ = self.protocol_sender.send(ProtocolMsg::RestartSync);
	}
}
*/

/// Substrate network service. Handles network IO and manages connectivity.
pub struct Service<B: BlockT + 'static, S: NetworkSpecialization<B>, I: IdentifySpecialization> {
	/// Sinks to propagate status updates.
	status_sinks: Arc<Mutex<Vec<mpsc::UnboundedSender<ProtocolStatus<B>>>>>,
	/// Are we connected to any peer?
	is_offline: Arc<AtomicBool>,
	/// Are we actively catching up with the chain?
	is_major_syncing: Arc<AtomicBool>,
	/// Peers whom we are connected with.
	peers: Arc<RwLock<HashMap<PeerId, ConnectedPeer<B>>>>,
//	/// Network service
//	network: Arc<Mutex<NetworkService<Message<B>, I>>>,
//	/// Peerset manager (PSM); manages the reputation of nodes and indicates the network which
//	/// nodes it should be connected to or not.
//	peerset: PeersetHandle,
	/// Protocol sender
	protocol_sender: Sender<ProtocolMsg<B, S>>,
//	/// Sender for messages to the background service task, and handle for the background thread.
//	/// Dropping the sender should close the task and the thread.
//	/// This is an `Option` because we need to extract it in the destructor.
//	bg_thread: Option<(oneshot::Sender<()>, thread::JoinHandle<()>)>,

	phantom: PhantomData<I>,
}

impl<B: BlockT + 'static, S: NetworkSpecialization<B>, I: IdentifySpecialization> Service<B, S, I> {
	/// Creates and register protocol with the network service
	pub fn new<H: ExHashT>(
		params: Params<B, S, H, I>,
		import_queue: Box<ImportQueue<B>>,
	) -> Result<(Arc<Service<B, S, I>>, NetworkChan<B>,  NetworkPort<B>, Sender<FromNetworkMsg<B>>, ImportQueuePort<B>), Error> {
		let (network_chan, network_port) = network_channel();
		let status_sinks = Arc::new(Mutex::new(Vec::new()));
		// Start in off-line mode, since we're not connected to any nodes yet.
		let is_offline = Arc::new(AtomicBool::new(true));
		let is_major_syncing = Arc::new(AtomicBool::new(false));
		let peers: Arc<RwLock<HashMap<PeerId, ConnectedPeer<B>>>> = Arc::new(Default::default());
		let (protocol_sender, network_to_protocol_sender) = Protocol::new(
			status_sinks.clone(),
			is_offline.clone(),
			is_major_syncing.clone(),
			peers.clone(),
			network_chan.clone(),
			params.config,
			params.chain,
			import_queue.clone(),
			params.on_demand,
			params.transaction_pool,
			params.specialization,
		)?;

		let service = Arc::new(Service {
			status_sinks,
			is_offline,
			is_major_syncing,
			peers,
			//peerset,
			//network,
			protocol_sender: protocol_sender.clone(),
			//bg_thread: Some(thread),
			phantom: PhantomData,
		});

		let (import_queue_sender, import_queue_port) = import_queue_channel();

		// connect the import-queue to the network service.
		let link = NetworkLink {
			protocol_sender,
			network_sender: network_chan.clone(),
			import_queue_sender,
		};

		import_queue.start(Box::new(link))?;

		Ok((service, network_chan, network_port, network_to_protocol_sender, import_queue_port))
	}

	/*
	/// Returns the downloaded bytes per second averaged over the past few seconds.
	#[inline]
	pub fn average_download_per_sec(&self) -> u64 {
		self.network.lock().average_download_per_sec()
	}

	/// Returns the uploaded bytes per second averaged over the past few seconds.
	#[inline]
	pub fn average_upload_per_sec(&self) -> u64 {
		self.network.lock().average_upload_per_sec()
	}

	/// Returns the network identity of the node.
	pub fn local_peer_id(&self) -> PeerId {
		self.network.lock().peer_id().clone()
	}
	*/

	/// Called when a new block is imported by the client.
	pub fn on_block_imported(&self, hash: B::Hash, header: B::Header) {
		let _ = self
			.protocol_sender
			.send(ProtocolMsg::BlockImported(hash, header));
	}

	/// Called when a new block is finalized by the client.
	pub fn on_block_finalized(&self, hash: B::Hash, header: B::Header) {
		let _ = self
			.protocol_sender
			.send(ProtocolMsg::BlockFinalized(hash, header));
	}

	/// Called when new transactons are imported by the client.
	pub fn trigger_repropagate(&self) {
		let _ = self.protocol_sender.send(ProtocolMsg::PropagateExtrinsics);
	}

	/// Make sure an important block is propagated to peers.
	///
	/// In chain-based consensus, we often need to make sure non-best forks are
	/// at least temporarily synced.
	pub fn announce_block(&self, hash: B::Hash) {
		let _ = self.protocol_sender.send(ProtocolMsg::AnnounceBlock(hash));
	}

	/// Send a consensus message through the gossip
	pub fn gossip_consensus_message(
		&self,
		topic: B::Hash,
		engine_id: ConsensusEngineId,
		message: Vec<u8>,
		force: bool,
	) {
		let _ = self
			.protocol_sender
			.send(ProtocolMsg::GossipConsensusMessage(
				topic, engine_id, message, force,
			));
	}

	/// Execute a closure with the chain-specific network specialization.
	pub fn with_spec<F>(&self, f: F)
		where F: FnOnce(&mut S, &mut Context<B>) + Send + 'static
	{
		let _ = self
			.protocol_sender
			.send(ProtocolMsg::ExecuteWithSpec(Box::new(f)));
	}

	/// Execute a closure with the consensus gossip.
	pub fn with_gossip<F>(&self, f: F)
		where F: FnOnce(&mut ConsensusGossip<B>, &mut Context<B>) + Send + 'static
	{
		let _ = self
			.protocol_sender
			.send(ProtocolMsg::ExecuteWithGossip(Box::new(f)));
	}

	/// Are we in the process of downloading the chain?
	/// Used by both SyncProvider and SyncOracle.
	fn is_major_syncing(&self) -> bool {
		self.is_major_syncing.load(Ordering::Relaxed)
	}
}

/// A link implementation that connects to the network.
#[derive(Clone)]
pub struct NetworkLink<B: BlockT, S: NetworkSpecialization<B>> {
	/// The protocol sender
	pub protocol_sender: Sender<ProtocolMsg<B, S>>,
	/// The network sender
	pub network_sender: NetworkChan<B>,
	/// The import queue sender
	pub import_queue_sender: ImportQueueChan<B>,
}

impl<B: BlockT, S: NetworkSpecialization<B>> Link<B> for NetworkLink<B, S> {
	fn block_imported(&self, hash: &B::Hash, number: NumberFor<B>) {
		let _ = self.protocol_sender.send(ProtocolMsg::BlockImportedSync(hash.clone(), number));
	}

	fn blocks_processed(&self, processed_blocks: Vec<B::Hash>, has_error: bool) {
		let _ = self.protocol_sender.send(ProtocolMsg::BlocksProcessed(processed_blocks.clone(), has_error));
		self.import_queue_sender.send(ImportQueueMsg::BlocksProcessed(processed_blocks, has_error));
	}

	fn justification_imported(&self, who: PeerId, hash: &B::Hash, number: NumberFor<B>, success: bool) {
		let _ = self.protocol_sender.send(ProtocolMsg::JustificationImportResult(hash.clone(), number, success));
		if !success {
			let reason = Severity::Bad(format!("Invalid justification provided for #{}", hash).to_string());
			let _ = self.network_sender.send(NetworkMsg::ReportPeer(who, reason));
		}
	}

	fn clear_justification_requests(&self) {
		let _ = self.protocol_sender.send(ProtocolMsg::ClearJustificationRequests);
	}

	fn request_justification(&self, hash: &B::Hash, number: NumberFor<B>) {
		let _ = self.protocol_sender.send(ProtocolMsg::RequestJustification(hash.clone(), number));
	}

	fn useless_peer(&self, who: PeerId, reason: &str) {
		trace!(target:"sync", "Useless peer {}, {}", who, reason);
		self.network_sender.send(NetworkMsg::ReportPeer(who, Severity::Useless(reason.to_string())));
	}

	fn note_useless_and_restart_sync(&self, who: PeerId, reason: &str) {
		trace!(target:"sync", "Bad peer {}, {}", who, reason);
		// is this actually malign or just useless?
		self.network_sender.send(NetworkMsg::ReportPeer(who, Severity::Useless(reason.to_string())));
		let _ = self.protocol_sender.send(ProtocolMsg::RestartSync);
	}

	fn restart(&self) {
		let _ = self.protocol_sender.send(ProtocolMsg::RestartSync);
	}
}

/// Create a ImportQueuePort/Chan pair.
pub fn import_queue_channel<B: BlockT + 'static>() -> (ImportQueueChan<B>, ImportQueuePort<B>) {
	let (import_queue_sender, import_queue_receiver) = channel::unbounded();
	let task_notify = Arc::new(AtomicTask::new());
	let import_queue_port = ImportQueuePort::new(import_queue_receiver, task_notify.clone());
	let import_queue_chan = ImportQueueChan::new(import_queue_sender, task_notify);
	(import_queue_chan, import_queue_port)
}


/// A sender of ProtocolMsg that notifies a task when a message has been sent.
#[derive(Clone)]
pub struct ImportQueueChan<B: BlockT + 'static> {
	sender: Sender<ImportQueueMsg<B>>,
	task_notify: Arc<AtomicTask>,
}

impl<B: BlockT + 'static> ImportQueueChan<B> {
	/// Create a new import_queue chan.
	pub fn new(sender: Sender<ImportQueueMsg<B>>, task_notify: Arc<AtomicTask>) -> Self {
		ImportQueueChan {
			sender,
			task_notify,
		}
	}

	/// Send a messaging, to be handled on a stream. Notify the task handling the stream.
	pub fn send(&self, msg: ImportQueueMsg<B>) {
		let _ = self.sender.send(msg);
		self.task_notify.notify();
	}
}

impl<B: BlockT + 'static> Drop for ImportQueueChan<B> {
	/// Notifying the task when a sender is dropped(when all are dropped, the stream is finished).
	fn drop(&mut self) {
		self.task_notify.notify();
	}
}


/// A receiver of ProtocolMsg that makes the protocol-id available with each message.
pub struct ImportQueuePort<B: BlockT + 'static> {
	receiver: Receiver<ImportQueueMsg<B>>,
	task_notify: Arc<AtomicTask>,
}

impl<B: BlockT + 'static> ImportQueuePort<B> {
	/// Create a new import_queue port for a given protocol-id.
	pub fn new(receiver: Receiver<ImportQueueMsg<B>>, task_notify: Arc<AtomicTask>) -> Self {
		Self {
			receiver,
			task_notify,
		}
	}

	/// Receive a message, if any is currently-enqueued.
	/// Register the current tokio task for notification when a new message is available.
	pub fn take_one_message(&self) -> Result<Option<ImportQueueMsg<B>>, ()> {
		self.task_notify.register();
		match self.receiver.try_recv() {
			Ok(msg) => Ok(Some(msg)),
			Err(TryRecvError::Empty) => Ok(None),
			Err(TryRecvError::Disconnected) => Err(()),
		}
	}

	/// Get a reference to the underlying crossbeam receiver.
	#[cfg(any(test, feature = "test-helpers"))]
	pub fn receiver(&self) -> &Receiver<ImportQueueMsg<B>> {
		&self.receiver
	}
}

pub enum ImportQueueMsg<B: BlockT> {
	/// A batch of blocks has been processed, with or without errors.
	BlocksProcessed(Vec<B::Hash>, bool),
}

/*
impl<B: BlockT + 'static, S: NetworkSpecialization<B>, I: IdentifySpecialization> ::consensus::SyncOracle for Service<B, S, I> {
	fn is_major_syncing(&self) -> bool {
		self.is_major_syncing()
	}

	fn is_offline(&self) -> bool {
		self.is_offline.load(Ordering::Relaxed)
	}
}
*/

/*
impl<B: BlockT + 'static, S: NetworkSpecialization<B>, I: IdentifySpecialization> Drop for Service<B, S, I> {
	fn drop(&mut self) {
		if let Some((sender, join)) = self.bg_thread.take() {
			let _ = sender.send(());
			if let Err(e) = join.join() {
				error!("Error while waiting on background thread: {:?}", e);
			}
		}
	}
}
*/

/*
impl<B: BlockT + 'static, S: NetworkSpecialization<B>, I: IdentifySpecialization> SyncProvider<B> for Service<B, S, I> {
	fn is_major_syncing(&self) -> bool {
		self.is_major_syncing()
	}

	/// Get sync status
	fn status(&self) -> mpsc::UnboundedReceiver<ProtocolStatus<B>> {
		let (sink, stream) = mpsc::unbounded();
		self.status_sinks.lock().push(sink);
		stream
	}

	fn network_state(&self) -> NetworkState {
		//self.network.lock().state()
		unreachable!()
	}

	fn peers(&self) -> Vec<(PeerId, PeerInfo<B>)> {
		let peers = (*self.peers.read()).clone();
		peers.into_iter().map(|(idx, connected)| (idx, connected.peer_info)).collect()
	}
}
*/

/*
/// Trait for managing network
pub trait ManageNetwork {
	/// Set to allow unreserved peers to connect
	fn accept_unreserved_peers(&self);
	/// Set to deny unreserved peers to connect
	fn deny_unreserved_peers(&self);
	/// Remove reservation for the peer
	fn remove_reserved_peer(&self, peer: PeerId);
	/// Add reserved peer
	fn add_reserved_peer(&self, peer: String) -> Result<(), String>;
}
*/

/*
impl<B: BlockT + 'static, S: NetworkSpecialization<B>, I: IdentifySpecialization> ManageNetwork for Service<B, S, I> {
	fn accept_unreserved_peers(&self) {
		self.peerset.set_reserved_only(false);
	}

	fn deny_unreserved_peers(&self) {
		self.peerset.set_reserved_only(true);
	}

	fn remove_reserved_peer(&self, peer: PeerId) {
		self.peerset.remove_reserved_peer(peer);
	}

	fn add_reserved_peer(&self, peer: String) -> Result<(), String> {
		let (peer_id, addr) = parse_str_addr(&peer).map_err(|e| format!("{:?}", e))?;
		self.peerset.add_reserved_peer(peer_id.clone());
		self.network.lock().add_known_address(peer_id, addr);
		Ok(())
	}
}
*/

/*
/// Create a NetworkPort/Chan pair.
pub fn network_channel<B: BlockT + 'static>() -> (NetworkChan<B>, NetworkPort<B>) {
	let (network_sender, network_receiver) = channel::unbounded();
	let task_notify = Arc::new(AtomicTask::new());
	let network_port = NetworkPort::new(network_receiver, task_notify.clone());
	let network_chan = NetworkChan::new(network_sender, task_notify);
	(network_chan, network_port)
}


/// A sender of NetworkMsg that notifies a task when a message has been sent.
#[derive(Clone)]
pub struct NetworkChan<B: BlockT + 'static> {
	sender: Sender<NetworkMsg<B>>,
	task_notify: Arc<AtomicTask>,
}

impl<B: BlockT + 'static> NetworkChan<B> {
	/// Create a new network chan.
	pub fn new(sender: Sender<NetworkMsg<B>>, task_notify: Arc<AtomicTask>) -> Self {
		 NetworkChan {
			sender,
			task_notify,
		}
	}

	/// Send a messaging, to be handled on a stream. Notify the task handling the stream.
	pub fn send(&self, msg: NetworkMsg<B>) {
		let _ = self.sender.send(msg);
		self.task_notify.notify();
	}
}

impl<B: BlockT + 'static> Drop for NetworkChan<B> {
	/// Notifying the task when a sender is dropped(when all are dropped, the stream is finished).
	fn drop(&mut self) {
		self.task_notify.notify();
	}
}


/// A receiver of NetworkMsg that makes the protocol-id available with each message.
pub struct NetworkPort<B: BlockT + 'static> {
	receiver: Receiver<NetworkMsg<B>>,
	task_notify: Arc<AtomicTask>,
}

impl<B: BlockT + 'static> NetworkPort<B> {
	/// Create a new network port for a given protocol-id.
	pub fn new(receiver: Receiver<NetworkMsg<B>>, task_notify: Arc<AtomicTask>) -> Self {
		Self {
			receiver,
			task_notify,
		}
	}

	/// Receive a message, if any is currently-enqueued.
	/// Register the current tokio task for notification when a new message is available.
	pub fn take_one_message(&self) -> Result<Option<NetworkMsg<B>>, ()> {
		self.task_notify.register();
		match self.receiver.try_recv() {
			Ok(msg) => Ok(Some(msg)),
			Err(TryRecvError::Empty) => Ok(None),
			Err(TryRecvError::Disconnected) => Err(()),
		}
	}

	/// Get a reference to the underlying crossbeam receiver.
	#[cfg(any(test, feature = "test-helpers"))]
	pub fn receiver(&self) -> &Receiver<NetworkMsg<B>> {
		&self.receiver
	}
}

/// Messages to be handled by NetworkService.
#[derive(Debug)]
pub enum NetworkMsg<B: BlockT + 'static> {
	/// Send an outgoing custom message.
	Outgoing(PeerId, Message<B>),
	/// Report a peer.
	ReportPeer(PeerId, Severity),
	/// Synchronization response.
	#[cfg(any(test, feature = "test-helpers"))]
	Synchronized,
}
*/