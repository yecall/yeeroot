
use runtime_primitives::{traits::{Block as BlockT, NumberFor}, ConsensusEngineId};
use crossbeam_channel::{self as channel, Receiver, Sender, TryRecvError};
use crate::protocol::FromNetworkMsg;
use crate::service::NetworkPort;
use futures::{Async, Future, Stream, stream, sync::oneshot, sync::mpsc};
use std::{io, thread};
use std::sync::Arc;
use parking_lot::{Mutex, RwLock};
use network_libp2p::{start_service, parse_str_addr, Service as NetworkService, ServiceEvent as NetworkServiceEvent};
use crate::message::Message;
use crate::error::Error;
use peerset::PeersetHandle;
use crate::IdentifySpecialization;

pub trait VNetworkProvider<B: BlockT + 'static, I: IdentifySpecialization>{

    fn start_thread(
        &self,
        shard_num: u16,
        protocol_sender: Sender<FromNetworkMsg<B>>,
        network_port: NetworkPort<B>) -> Result<(), Error>;
}