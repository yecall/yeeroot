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

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{io, thread};
use log::{warn, debug, error, trace, info};
use futures::{Async, Future, Stream, stream, sync::oneshot, sync::mpsc};
use parking_lot::{Mutex, RwLock};
use network_libp2p::{ProtocolId, NetworkConfiguration, Severity};
use network_libp2p::{start_service, parse_str_addr, Service as NetworkService, ServiceEvent as NetworkServiceEvent};
use network_libp2p::{RegisteredProtocol, NetworkState};
use network_libp2p::ForeignPeersetHandle;
use crate::message::Message;
use crate::protocol::{self, Protocol, FromNetworkMsg, ProtocolMsg};
use crate::config::Params;
use crossbeam_channel::{self as channel, Receiver, Sender, TryRecvError};
use crate::error::Error;
use runtime_primitives::{traits::{Block as BlockT, NumberFor, Header}, ConsensusEngineId};
use crate::{IdentifySpecialization, identify_specialization::ForeignIdentifySpecialization};

use tokio::prelude::task::AtomicTask;
use tokio::runtime::Builder as RuntimeBuilder;

pub use network_libp2p::PeerId;
use serde::export::PhantomData;
use crate::message::generic::{Message as GenericMessage, OutMessage, BestBlockInfo};
use crate::chain::Client;
use regex::Regex;

/// Sync status
pub trait SyncProvider<B: BlockT, H: ExHashT>: Send + Sync {

	fn on_relay_extrinsics(&self, shard_num: u16, extrinsics: Vec<(H, B::Extrinsic)>);

	fn out_messages(&self) -> mpsc::UnboundedReceiver<OutMessage<B>>;

	/// Get network state.
	fn network_state(&self) -> NetworkState;

	// Get client info.
	fn client_info(&self) -> HashMap<u16, Option<client::ClientInfo<B>>>;
}

/// Minimum Requirements for a Hash within Networking
pub trait ExHashT:
	::std::hash::Hash + Eq + ::std::fmt::Debug + Clone + Send + Sync + 'static
{
}
impl<T> ExHashT for T where
	T: ::std::hash::Hash + Eq + ::std::fmt::Debug + Clone + Send + Sync + 'static
{
}

/// Substrate network service. Handles network IO and manages connectivity.
pub struct Service<B: BlockT + 'static, I: IdentifySpecialization, H: ExHashT> {
	/// Sinks to propagate out messages.
	out_message_sinks: Arc<Mutex<Vec<mpsc::UnboundedSender<OutMessage<B>>>>>,
	/// Are we connected to any peer?
	is_offline: Arc<AtomicBool>,
	/// Are we actively catching up with the chain?
	is_major_syncing: Arc<AtomicBool>,
	/// Network service
	network: Arc<Mutex<NetworkService<Message<B>, I>>>,
	/// Peerset manager (PSM); manages the reputation of nodes and indicates the network which
	/// nodes it should be connected to or not.
	peerset: ForeignPeersetHandle,
	/// Protocol sender
	protocol_sender: Sender<ProtocolMsg<B, H>>,
	/// Sender for messages to the background service task, and handle for the background thread.
	/// Dropping the sender should close the task and the thread.
	/// This is an `Option` because we need to extract it in the destructor.
	bg_thread: Option<(oneshot::Sender<()>, thread::JoinHandle<()>)>,

	vnetwork_holder: VNetworkHolder<B, I>,

	vnetwork_bg_thread: Option<(oneshot::Sender<()>, thread::JoinHandle<()>)>,

	/// self full node sharding number.
	shard_num: u16,

	shard_count: u16,

	chain: Arc<Client<B>>,
}

impl<B: BlockT + 'static, I: IdentifySpecialization, H: ExHashT> Service<B, I, H> {
	/// Creates and register protocol with the network service
	pub fn new(
		params: Params<B, I>,
		protocol_id: ProtocolId,
	) -> Result<(Arc<Service<B, I, H>>, NetworkChan<B>), Error> {
		let shard_num = params.network_config.shard_num;
		let shard_count = params.network_config.shard_count;
		let (network_chan, network_port) = network_channel();
		let (from_network_chan, from_network_port) = from_network_channel();
		let out_message_sinks = Arc::new(Mutex::new(Vec::new()));
		// Start in off-line mode, since we're not connected to any nodes yet.
		let is_offline = Arc::new(AtomicBool::new(true));
		let is_major_syncing = Arc::new(AtomicBool::new(false));
		let (protocol_sender, network_to_protocol_sender) = Protocol::<_, H>::new(
			out_message_sinks.clone(),
			is_offline.clone(),
			is_major_syncing.clone(),
			network_chan.clone(),
			params.config,
			params.chain.clone(),
		)?;
		let versions = [(protocol::CURRENT_VERSION as u8)];
		let registered = RegisteredProtocol::new(protocol_id, &versions[..]);
		let (thread, network, peerset) = start_thread(
			network_to_protocol_sender,
			network_port,
			params.network_config,
			registered,
			params.identify_specialization,
			from_network_chan.clone(),
			params.chain.clone(),
			protocol_sender.clone(),
		)?;

		let vnetwork_holder = VNetworkHolder{
			peerset: peerset.clone(),
			network_service: network.clone(),
			network_port_list: Arc::new(RwLock::new(HashMap::new())),
			from_network_port: Arc::new(from_network_port),
			protocol_sender_list: Arc::new(RwLock::new(HashMap::new())),
			chain_list: Arc::new(RwLock::new(HashMap::new())),
			from_network_chan,
			import_queue_port_list: Arc::new(RwLock::new(HashMap::new())),
			out_message_sinks: out_message_sinks.clone(),
			network_count: Arc::new(RwLock::new(0)),
			network_ready: Arc::new(RwLock::new(false)),
		};
		let vnetwork_thread = vnetwork_holder.start_thread()?;

		let service = Arc::new(Service {
			out_message_sinks,
			is_offline,
			is_major_syncing,
			peerset,
			network,
			protocol_sender: protocol_sender.clone(),
			bg_thread: Some(thread),
			vnetwork_holder,
			vnetwork_bg_thread: Some(vnetwork_thread),
			shard_num,
			shard_count,
			chain: params.chain,
		});

		Ok((service, network_chan))
	}

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

	/// Called when new relay extrinsics generated.
	pub fn on_relay_extrinsics(&self, shard_num: u16, extrinsics: Vec<(H, B::Extrinsic)>) {
		let _ = self
			.protocol_sender
			.send(ProtocolMsg::RelayExtrinsics(shard_num, extrinsics));
	}

	/// Are we in the process of downloading the chain?
	/// Used by both SyncProvider and SyncOracle.
	fn is_major_syncing(&self) -> bool {
		self.is_major_syncing.load(Ordering::Relaxed)
	}
}

impl<B: BlockT + 'static, I: IdentifySpecialization, H: ExHashT> Drop for Service<B, I, H> {
	fn drop(&mut self) {
		info!("Foreign network service dropping");
		if let Some((sender, join)) = self.bg_thread.take() {
			let _ = sender.send(());
			if let Err(e) = join.join() {
				error!("Error while waiting on background thread: {:?}", e);
			}
		}
		if let Some((sender, join)) = self.vnetwork_bg_thread.take() {
			let _ = sender.send(());
			if let Err(e) = join.join() {
				error!("Error while waiting on vnetwork background thread: {:?}", e);
			}
		}
	}
}

impl<B: BlockT + 'static, I: IdentifySpecialization, H: ExHashT> SyncProvider<B, H> for Service<B, I, H> {

	fn on_relay_extrinsics(&self, shard_num: u16, extrinsics: Vec<(H, B::Extrinsic)>){
		self.on_relay_extrinsics(shard_num, extrinsics);
	}

	fn out_messages(&self) -> mpsc::UnboundedReceiver<OutMessage<B>>{
		let (sink, stream) = mpsc::unbounded();
		self.out_message_sinks.lock().push(sink);
		stream
	}

	fn network_state(&self) -> NetworkState {

		self.network.lock().state()
	}

	fn client_info(&self) -> HashMap<u16, Option<client::ClientInfo<B>>>{
		// foreign network client
		let mut info : HashMap<u16, Option<client::ClientInfo<B>>> =  self.vnetwork_holder.chain_list.read().iter()
			.map(|(shard_num, client)|(*shard_num, client.info().ok()))
			.collect();
		// self full node client
		info.insert(self.shard_num, self.chain.info().ok());

		info
	}
}

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

/// Starts the background thread that handles the networking.
fn start_thread<B: BlockT + 'static, I: IdentifySpecialization, H: ExHashT>(
	protocol_sender: Sender<FromNetworkMsg<B>>,
	network_port: NetworkPort<B>,
	config: NetworkConfiguration,
	registered: RegisteredProtocol<Message<B>>,
	identify_specialization: I,
	from_network_chan: FromNetworkChan<B>,
	chain: Arc<Client<B>>,
	protocol_msg_sender: Sender<ProtocolMsg<B, H>>,
) -> Result<((oneshot::Sender<()>, thread::JoinHandle<()>), Arc<Mutex<NetworkService<Message<B>, I>>>, ForeignPeersetHandle), Error> {
	// Start the main service.
	let (service, peerset) = match start_service(config, registered, identify_specialization) {
		Ok((service, peerset)) => (Arc::new(Mutex::new(service)), peerset),
		Err(err) => {
			warn!("Error starting network: {}", err);
			return Err(err.into())
		},
	};

	let (close_tx, close_rx) = oneshot::channel();
	let service_clone = service.clone();
	let mut runtime = RuntimeBuilder::new().name_prefix("libp2p-").build()?;
	let peerset_clone = peerset.clone();
	let thread = thread::Builder::new().name("network".to_string()).spawn(move || {
		let fut = run_thread(protocol_sender, service_clone, network_port,
							 peerset_clone, from_network_chan, chain, protocol_msg_sender)
			.select(close_rx.then(|_| Ok(())))
			.map(|(val, _)| val)
			.map_err(|(err,_ )| err);

		// Note that we use `block_on` and not `block_on_all` because we want to kill the thread
		// instantly if `close_rx` receives something.
		match runtime.block_on(fut) {
			Ok(()) => debug!(target: "sub-libp2p-foreign", "Networking thread finished"),
			Err(err) => error!(target: "sub-libp2p-foreign", "Error while running libp2p: {:?}", err),
		};
	})?;

	Ok(((close_tx, thread), service, peerset))
}

/// Runs the background thread that handles the networking.
fn run_thread<B: BlockT + 'static, I: IdentifySpecialization, H: ExHashT>(
	protocol_sender: Sender<FromNetworkMsg<B>>,
	network_service: Arc<Mutex<NetworkService<Message<B>, I>>>,
	network_port: NetworkPort<B>,
	peerset: ForeignPeersetHandle,
	from_network_chan: FromNetworkChan<B>,
	chain: Arc<Client<B>>,
	protocol_msg_sender: Sender<ProtocolMsg<B, H>>,
) -> impl Future<Item = (), Error = io::Error> {

	let network_service_2 = network_service.clone();

	// Protocol produces a stream of messages about what happens in sync.
	let protocol = stream::poll_fn(move || {
		match network_port.take_one_message() {
			Ok(Some(message)) => Ok(Async::Ready(Some(message))),
			Ok(None) => Ok(Async::NotReady),
			Err(_) => Err(())
		}
	}).for_each(move |msg| {
		// Handle message from Protocol.
		match msg {
			NetworkMsg::Outgoing(who, outgoing_message) => {
				network_service_2
					.lock()
					.send_custom_message(&who, outgoing_message);
			},
			NetworkMsg::ReportPeer(who, severity) => {
				match severity {
					Severity::Bad(message) => {
						debug!(target: "sync-foreign", "Banning {:?} because {:?}", who, message);
						network_service_2.lock().drop_node(&who);
						// temporary: make sure the peer gets dropped from the peerset
						peerset.report_peer(who, i32::min_value());
					},
					Severity::Useless(message) => {
						debug!(target: "sync-foreign", "Dropping {:?} because {:?}", who, message);
						network_service_2.lock().drop_node(&who)
					},
					Severity::Timeout => {
						debug!(target: "sync-foreign", "Dropping {:?} because it timed out", who);
						network_service_2.lock().drop_node(&who)
					},
				}
			},
			#[cfg(any(test, feature = "test-helpers"))]
			NetworkMsg::Synchronized => (),
		}
		Ok(())
	})
	.then(|res| {
		match res {
			Ok(()) => (),
			Err(_) => error!("Protocol disconnected"),
		};
		Ok(())
	});

	// The network service produces events about what happens on the network. Let's process them.
	let network = stream::poll_fn(move || network_service.lock().poll()).for_each(move |event| {
		match event {
			NetworkServiceEvent::OpenedCustomProtocol { peer_id, version, debug_info, .. } => {
				debug_assert_eq!(version, protocol::CURRENT_VERSION as u8);
				from_network_chan.send(FromNetworkMsg::PeerConnected(peer_id.clone(), debug_info.clone()));
				let _ = protocol_sender.send(FromNetworkMsg::PeerConnected(peer_id, debug_info));
			}
			NetworkServiceEvent::ClosedCustomProtocol { peer_id, debug_info, .. } => {
				from_network_chan.send(FromNetworkMsg::PeerDisconnected(peer_id.clone(), debug_info.clone()));
				let _ = protocol_sender.send(FromNetworkMsg::PeerDisconnected(peer_id, debug_info));
			}
			NetworkServiceEvent::CustomMessage { peer_id, message, .. } => {
				from_network_chan.send(FromNetworkMsg::CustomMessage(peer_id.clone(), message.clone()));
				let _ = protocol_sender.send(FromNetworkMsg::CustomMessage(peer_id, message));
				return Ok(())
			}
			NetworkServiceEvent::Clogged { peer_id, messages, .. } => {
				debug!(target: "sync-foreign", "{} clogging messages:", messages.len());
				for msg in messages.into_iter().take(5) {
					debug!(target: "sync-foreign", "{:?}", msg);
					from_network_chan.send(FromNetworkMsg::PeerClogged(peer_id.clone(), Some(msg.clone())));
					let _ = protocol_sender.send(FromNetworkMsg::PeerClogged(peer_id.clone(), Some(msg)));
				}
			}
		};
		Ok(())
	});

	let client = chain.client_import_notification_stream().for_each(move |event|{

		let hash = event.hash;
		let header = event.header;

		protocol_msg_sender.send(ProtocolMsg::BlockImported(hash, header)).map_err(|e|{
			error!("Client error: {:?}", e)
		})?;

		Ok(())
	}).then(|res| {
		match res {
			Ok(()) => (),
			Err(_) => error!("Client error"),
		};
		Ok(())
	});

	// Merge all futures into one.
	let futures: Vec<Box<Future<Item = (), Error = io::Error> + Send>> = vec![
		Box::new(protocol) as Box<_>,
		Box::new(network) as Box<_>,
		Box::new(client) as Box<_>
	];

	futures::select_all(futures)
		.and_then(move |_| {
			debug!("Networking ended");
			Ok(())
		})
		.map_err(|(r, _, _)| r)
}

/// Create a NetworkPort/Chan pair.
pub fn from_network_channel<B: BlockT + 'static>() -> (FromNetworkChan<B>, FromNetworkPort<B>) {
	let (from_network_sender, from_network_receiver) = channel::unbounded();
	let task_notify = Arc::new(AtomicTask::new());
	let from_network_port = FromNetworkPort::new(from_network_receiver, task_notify.clone());
	let from_network_chan = FromNetworkChan::new(from_network_sender, task_notify);
	(from_network_chan, from_network_port)
}

/// A sender of NetworkMsg that notifies a task when a message has been sent.
#[derive(Clone)]
pub struct FromNetworkChan<B: BlockT + 'static> {
	sender: Sender<FromNetworkMsg<B>>,
	task_notify: Arc<AtomicTask>,
}

impl<B: BlockT + 'static> FromNetworkChan<B> {
	/// Create a new network chan.
	pub fn new(sender: Sender<FromNetworkMsg<B>>, task_notify: Arc<AtomicTask>) -> Self {
		FromNetworkChan {
			sender,
			task_notify,
		}
	}

	/// Send a messaging, to be handled on a stream. Notify the task handling the stream.
	pub fn send(&self, msg: FromNetworkMsg<B>) {
		let _ = self.sender.send(msg);
		self.task_notify.notify();
	}
}

impl<B: BlockT + 'static> Drop for FromNetworkChan<B> {
	/// Notifying the task when a sender is dropped(when all are dropped, the stream is finished).
	fn drop(&mut self) {
		self.task_notify.notify();
	}
}


/// A receiver of NetworkMsg that makes the protocol-id available with each message.
pub struct FromNetworkPort<B: BlockT + 'static> {
	receiver: Receiver<FromNetworkMsg<B>>,
	task_notify: Arc<AtomicTask>,
}

impl<B: BlockT + 'static> FromNetworkPort<B> {
	/// Create a new network port for a given protocol-id.
	pub fn new(receiver: Receiver<FromNetworkMsg<B>>, task_notify: Arc<AtomicTask>) -> Self {
		Self {
			receiver,
			task_notify,
		}
	}

	/// Receive a message, if any is currently-enqueued.
	/// Register the current tokio task for notification when a new message is available.
	pub fn take_one_message(&self) -> Result<Option<FromNetworkMsg<B>>, ()> {
		self.task_notify.register();
		match self.receiver.try_recv() {
			Ok(msg) => Ok(Some(msg)),
			Err(TryRecvError::Empty) => Ok(None),
			Err(TryRecvError::Disconnected) => Err(()),
		}
	}

	/// Get a reference to the underlying crossbeam receiver.
	#[cfg(any(test, feature = "test-helpers"))]
	pub fn receiver(&self) -> &Receiver<FromNetworkMsg<B>> {
		&self.receiver
	}
}

struct VNetworkHolder<B: BlockT + 'static, I: IdentifySpecialization>{

	peerset: ForeignPeersetHandle,
	network_service: Arc<Mutex<NetworkService<Message<B>, I>>>,
	network_port_list: Arc<RwLock<HashMap<u16, substrate_network::service::NetworkPort<B>>>>,
	from_network_port: Arc<FromNetworkPort<B>>,
	protocol_sender_list: Arc<RwLock<HashMap<u16, Sender<substrate_network::protocol::FromNetworkMsg<B>>>>>,
	chain_list: Arc<RwLock<HashMap<u16, Arc<substrate_network::chain::Client<B>>>>>,
	from_network_chan: FromNetworkChan<B>,
	import_queue_port_list: Arc<RwLock<HashMap<u16, vnetwork::ImportQueuePort<B>>>>,
	out_message_sinks: Arc<Mutex<Vec<mpsc::UnboundedSender<OutMessage<B>>>>>,
	network_count: Arc<RwLock<u16>>,
	network_ready: Arc<RwLock<bool>>,
}

impl<B: BlockT + 'static, I: IdentifySpecialization> VNetworkHolder<B, I>{

	fn start_thread(&self) -> Result<(oneshot::Sender<()>, thread::JoinHandle<()>), Error> {

		let (close_tx, close_rx) = oneshot::channel();
		let mut runtime = RuntimeBuilder::new().name_prefix("vnetwork-").build()?;

		let peerset = self.peerset.clone();
		let network_service = self.network_service.clone();
		let network_port_list = self.network_port_list.clone();
		let from_network_port = self.from_network_port.clone();
		let protocol_sender_list = self.protocol_sender_list.clone();
		let from_network_chan = self.from_network_chan.clone();
		let import_queue_port_list = self.import_queue_port_list.clone();
		let chain_list = self.chain_list.clone();
		let out_message_sinks = self.out_message_sinks.clone();
		let network_ready = self.network_ready.clone();

		let thread = thread::Builder::new().name("vnetwork".to_string()).spawn(move || {
			let fut = Self::run_thread(
				peerset,
				network_service,
				network_port_list,
				from_network_port,
				protocol_sender_list,
				from_network_chan,
				import_queue_port_list,
				chain_list,
				out_message_sinks,
				network_ready,
			)
				.select(close_rx.then(|_| Ok(())))
				.map(|(val, _)| val)
				.map_err(|(err,_ )| err);

			// Note that we use `block_on` and not `block_on_all` because we want to kill the thread
			// instantly if `close_rx` receives something.
			match runtime.block_on(fut) {
				Ok(()) => debug!(target: "sub-libp2p-foreign", "VNetworking thread finished"),
				Err(err) => error!(target: "sub-libp2p-foreign", "Error while running libp2p: {:?}", err),
			};
		})?;

		Ok((close_tx, thread))
	}

	/// Runs the background thread that handles the networking.
	fn run_thread(
		peerset: ForeignPeersetHandle,
		network_service: Arc<Mutex<NetworkService<Message<B>, I>>>,
		network_port_list: Arc<RwLock<HashMap<u16, substrate_network::service::NetworkPort<B>>>>,
		from_network_port: Arc<FromNetworkPort<B>>,
		protocol_sender_list: Arc<RwLock<HashMap<u16, Sender<substrate_network::protocol::FromNetworkMsg<B>>>>>,
		from_network_chan: FromNetworkChan<B>,
		import_queue_port_list: Arc<RwLock<HashMap<u16, vnetwork::ImportQueuePort<B>>>>,
		chain_list: Arc<RwLock<HashMap<u16, Arc<substrate_network::chain::Client<B>>>>>,
		out_message_sinks: Arc<Mutex<Vec<mpsc::UnboundedSender<OutMessage<B>>>>>,
		network_ready: Arc<RwLock<bool>>,
	) -> impl Future<Item = (), Error = io::Error> {

		let peerset_router = peerset.router();

		// Protocol produces a stream of messages about what happens in sync.
		let protocol = stream::poll_fn(move || {

			let mut error_shard_num = None;
			for (shard_num, network_port) in &*network_port_list.read(){
				let shard_num = *shard_num;
				match network_port.take_one_message() {
					Ok(Some(message)) => return Ok(Async::Ready(Some((shard_num, message)))),
					Ok(None) => {},
					Err(_) => {
						error_shard_num = Some(shard_num);
						break;
					}
				}
			}
			if let Some(error_shard_num) = error_shard_num{
				network_port_list.write().remove(&error_shard_num);
			}

			Ok(Async::NotReady)

		}).for_each(move |(shard_num, msg)| {
			// Handle message from Protocol.
			match msg {
				substrate_network::NetworkMsg::Outgoing(who, outgoing_message) => {
					network_service
						.lock()
						.send_custom_message(&who,  GenericMessage::VMessage(shard_num, outgoing_message));
				},
				substrate_network::NetworkMsg::ReportPeer(who, severity) => {
					match severity {
						substrate_network::Severity::Bad(message) => {
							debug!(target: "sync-foreign", "Banning {:?} because {:?}", who, message);
							network_service.lock().drop_node(&who);
							// temporary: make sure the peer gets dropped from the peerset
							peerset.report_peer(who, i32::min_value());
						},
						substrate_network::Severity::Useless(message) => {
							debug!(target: "sync-foreign", "Dropping {:?} because {:?}", who, message);
							network_service.lock().drop_node(&who)
						},
						substrate_network::Severity::Timeout => {
							debug!(target: "sync-foreign", "Dropping {:?} because it timed out", who);
							network_service.lock().drop_node(&who)
						},
					}
				},
				#[cfg(any(test, feature = "test-helpers"))]
				substrate_network::NetworkMsg::Synchronized => (),
			}
			Ok(())
		}).then(|res : Result<(), io::Error> | {
				match res {
					Ok(()) => (),
					Err(_) => error!("Protocol disconnected"),
				};
				Ok(())
			});

		// Protocol produces a stream of messages about what happens in sync.
		let network = stream::poll_fn(move || {
			match from_network_port.take_one_message() {
				Ok(Some(message)) => Ok(Async::Ready(Some(message))),
				Ok(None) => Ok(Async::NotReady),
				Err(_) => Err(())
			}
		}).for_each(move |msg| {
			// Handle message from network.
			match msg {
				FromNetworkMsg::PeerConnected(peer_id, debug_info) => {
					if let Some(shard_num) = peerset_router.get_shard_num(&peer_id){
						let keys : Vec<u16> = protocol_sender_list.read().keys().cloned().collect();
						if let Some(sender) = protocol_sender_list.read().get(&shard_num){
							sender.send(vnetwork::FromNetworkMsg::PeerConnected(peer_id, debug_info)).map_err(|_|())?;
						}else if !*network_ready.read() {
							debug!(target: "sync-foreign", "Place FromNetworkMsg PeerConnected back since sender not ready: shard_num: {}", shard_num);
							from_network_chan.send(FromNetworkMsg::PeerConnected(peer_id, debug_info));
						}
					}
				},
				FromNetworkMsg::PeerDisconnected(peer_id, debug_info) => {
					if let Some(shard_num) = peerset_router.get_shard_num(&peer_id){
						if let Some(sender) = protocol_sender_list.read().get(&shard_num){
							sender.send(vnetwork::FromNetworkMsg::PeerDisconnected(peer_id, debug_info)).map_err(|_|())?;
						}else if !*network_ready.read() {
							debug!(target: "sync-foreign", "Place FromNetworkMsg PeerDisconnected back since sender not ready: shard_num: {}", shard_num);
							from_network_chan.send(FromNetworkMsg::PeerDisconnected(peer_id, debug_info));
						}
					}
				},
				FromNetworkMsg::CustomMessage(peer_id, message) => {
					if let GenericMessage::VMessage(vmessage_shard_num, vmessage) = message{
						if let Some(shard_num) = peerset_router.get_shard_num(&peer_id){
							if vmessage_shard_num == shard_num {
								if let Some(sender) = protocol_sender_list.read().get(&shard_num) {
									sender.send(vnetwork::FromNetworkMsg::CustomMessage(peer_id, vmessage)).map_err(|_| ())?;
								}else if !*network_ready.read() {
									debug!(target: "sync-foreign", "Place FromNetworkMsg CustomMessage back since sender not ready: shard_num: {}", shard_num);
									from_network_chan.send(FromNetworkMsg::CustomMessage(peer_id, GenericMessage::VMessage(vmessage_shard_num, vmessage)));
								}
							}
						}
					}
				},
				FromNetworkMsg::PeerClogged(peer_id, message) => {
					if let Some(GenericMessage::VMessage(vmessage_shard_num, vmessage)) = message{
						if let Some(shard_num) = peerset_router.get_shard_num(&peer_id){
							if vmessage_shard_num == shard_num {
								if let Some(sender) = protocol_sender_list.read().get(&shard_num) {
									sender.send(vnetwork::FromNetworkMsg::PeerClogged(peer_id, Some(vmessage))).map_err(|_| ())?;
								}else if !*network_ready.read() {
									debug!(target: "sync-foreign", "Place FromNetworkMsg PeerClogged back since sender not ready: shard_num: {}", shard_num);
									from_network_chan.send(FromNetworkMsg::PeerClogged(peer_id, Some(GenericMessage::VMessage(vmessage_shard_num, vmessage))));
								}
							}
						}
					}
				},
				#[cfg(any(test, feature = "test-helpers"))]
				Synchronize => (),
			}
			Ok(())
		}).then(|res | {
			match res {
				Ok(()) => (),
				Err(_) => error!("Network disconnected"),
			};
			Ok(())
		});

		let import_queue = stream::poll_fn(move || {

			let mut error_shard_num = None;
			for (shard_num, import_queue_port) in &*import_queue_port_list.read(){
				let shard_num = *shard_num;
				match import_queue_port.take_one_message() {
					Ok(Some(message)) => return Ok(Async::Ready(Some((shard_num, message)))),
					Ok(None) => {},
					Err(_) => {
						error_shard_num = Some(shard_num);
						break;
					}
				}
			}
			if let Some(error_shard_num) = error_shard_num{
				import_queue_port_list.write().remove(&error_shard_num);
			}

			Ok(Async::NotReady)

		}).for_each(move |(shard_num, msg)| {
			// Handle message from Protocol.
			match msg {
				vnetwork::ImportQueueMsg::BlocksProcessed(hashes, has_error) => {
					debug!(target: "sync-foreign", "Blocks processed: shard_num:{}, hashes: {:?}, has_error={}", shard_num, hashes, has_error);
					if let Some(chain) = chain_list.read().get(&shard_num) {
						if let Ok(info) = chain.info() {
							let chain_info = info.chain;
							let out_message = OutMessage::BestBlockInfoChanged(shard_num, BestBlockInfo {
								best_number: chain_info.best_number,
								best_hash: chain_info.best_hash,
							});
							debug!(target: "sync-foreign", "Send out message: {:?}", out_message);
							out_message_sinks.lock().retain(|sink| sink.unbounded_send(out_message.clone()).is_ok());
						}
					}
				}
			}
			Ok(())
		}).then(|res : Result<(), io::Error> | {
			match res {
				Ok(()) => (),
				Err(_) => error!("import queue disconnected"),
			};
			Ok(())
		});

		// Merge all futures into one.
		let futures: Vec<Box<Future<Item = (), Error = io::Error> + Send>> = vec![
			Box::new(protocol) as Box<_>,
			Box::new(network) as Box<_>,
			Box::new(import_queue) as Box<_>,
		];

		futures::select_all(futures)
			.and_then(move |_| {
				debug!("Networking ended");
				Ok(())
			})
			.map_err(|(r, _, _)| r)
	}
}

use substrate_service::{Components, FactoryBlock, ComponentExHash};

impl<F, I, EH> substrate_service::NetworkProvider<F, EH> for Service<FactoryBlock<F>, I, EH> where
	F: substrate_service::ServiceFactory,
	I: IdentifySpecialization,
	EH: substrate_network::ExHashT,
{
	fn provide_network(
		&self,
		network_id: u32,
		params: substrate_service::NetworkProviderParams<F, EH>,
		protocol_id: substrate_network::ProtocolId,
		import_queue: Box<dyn consensus::import_queue::ImportQueue<substrate_service::FactoryBlock<F>>>,
	) -> Result<substrate_network::NetworkChan<substrate_service::FactoryBlock<F>>, substrate_network::Error>{

		let shard_num = network_id as u16;

		let chain = params.chain.clone();

		let (service, network_chan,
			network_port, network_to_protocol_sender, import_queue_port) =
			vnetwork::Service::new(params, import_queue)?;

		self.vnetwork_holder.network_port_list.write().entry(shard_num).or_insert(network_port);
		self.vnetwork_holder.protocol_sender_list.write().entry(shard_num).or_insert(network_to_protocol_sender);
		self.vnetwork_holder.chain_list.write().entry(shard_num).or_insert(chain);
		self.vnetwork_holder.import_queue_port_list.write().entry(shard_num).or_insert(import_queue_port);

		let mut network_count = self.vnetwork_holder.network_count.write();
		*network_count = *network_count + 1;
		if *network_count == self.shard_count - 1 {
			let mut network_ready = self.vnetwork_holder.network_ready.write();
			*network_ready = true;
		}

		Ok(network_chan)
	}

}
