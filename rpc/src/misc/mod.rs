use std::sync::Arc;
use std::hash::Hasher;
use parity_codec::{Encode, Decode};
use jsonrpc_derive::rpc;
use jsonrpc_core::BoxFuture;
use crate::errors;
use jsonrpc_core::futures::future::{self, Future, IntoFuture};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde_json::Value;
use std::marker::PhantomData;
use yee_primitives::RecommitRelay;
use futures::sync::mpsc;
use parking_lot::RwLock;
use crfg::CrfgState;
use std::time::Duration;
use substrate_primitives::Bytes;
use transaction_pool::txpool::{Pool, ChainApi as PoolChainApi};

#[rpc]
pub trait MiscApi<Hash, Number> {
	#[rpc(name = "author_recommitRelay")]
	fn recommit_relay_extrinsic(&self, hash: Hash, index: usize) -> errors::Result<()>;

	#[rpc(name = "author_waitingExtrinsics")]
	fn waiting_extrinsics(&self) -> errors::Result<Vec<Bytes>>;

	#[rpc(name = "crfg_state")]
	fn crfg_state(&self) -> errors::Result<Option<types::CrfgState<Hash, Number>>>;
}

pub struct Misc<Hash, Number, P: PoolChainApi> {
	recommit_relay_sender: Arc<Option<mpsc::UnboundedSender<RecommitRelay<Hash>>>>,
	crfg_state: Arc<RwLock<Option<CrfgState<Hash, Number>>>>,
	pool: Arc<Pool<P>>,
}

impl<Hash, Number> Misc<Hash, Number> {
	pub fn new(
		recommit_relay_sender: Arc<RwLock<Option<mpsc::UnboundedSender<RecommitRelay<Hash>>>>>,
		crfg_state: Arc<RwLock<Option<CrfgState<Hash, Number>>>>,
		pool: Arc<Pool<P>>,
	) -> Self {
		Self {
			recommit_relay_sender,
			crfg_state,
			pool,
		}
	}
}

impl<Hash, Number, P> MiscApi<Hash, Number> for Misc<Hash, Number, P> where
	Hash: Send + Clone + Sync + 'static,
	Number: Send + Clone + Sync + 'static,
	P: PoolChainApi + Sync + Send + 'static,
{
	fn recommit_relay_extrinsic(&self, hash: Hash, index: usize) -> errors::Result<()> {
		let recommit_param = RecommitRelay {
			hash,
			index,
        };
		if let Some(sender) =  self.recommit_relay_sender.write().as_ref() {
			sender.unbounded_send(recommit_param);
			Ok(())
		} else {
			Err(errors::Error::from(errors::ErrorKind::RecommitFailed).into())
		}
	}

	fn waiting_extrinsics(&self) -> errors::Result<Vec<Bytes>> {
		Ok(self.pool.futures().map(|tx| tx.data.encode().into()).collect())
	}

	fn crfg_state(&self) -> errors::Result<Option<types::CrfgState<Hash, Number>>> {
		let state = self.crfg_state.read().as_ref().cloned().map(|x|x.into());
		Ok(state)
	}
}

mod types {
	use std::sync::Arc;
	use yee_runtime::{AuthorityId, BlockNumber};
	use std::time::Duration;
	use parity_codec::alloc::collections::HashMap;
	use serde::Serialize;
	use yee_serde_hex::SerdeHex;
	use substrate_primitives::crypto::Pair;

	#[derive(Serialize, Eq, PartialEq, Hash)]
	pub struct Public(#[serde(with = "SerdeHex")] Vec<u8>);

	#[derive(Serialize)]
	pub struct CrfgState<H, N> {
		pub config: Config,
		pub set_id: u64,
		pub voters: VoterSet,
		pub set_status: VoterSetState<H, N>,
	}

	#[derive(Serialize)]
	pub struct Config {
		pub gossip_duration: Duration,
		pub justification_period: u64,
		pub local_key_public: Option<Public>,
		pub local_next_key_public: Option<Public>,
		pub name: Option<String>,
	}

	#[derive(Serialize)]
	pub struct VoterInfo {
		canon_idx: usize,
		weight: u64,
	}

	#[derive(Serialize)]
	pub struct VoterSet {
		weights: HashMap<Public, VoterInfo>,
		voters: Vec<(Public, u64)>,
		threshold: u64,
	}

	#[derive(Serialize)]
	pub enum VoterSetState<H, N> {
		Paused(u64, RoundState<H, N>),
		Live(u64, RoundState<H, N>),
	}

	#[derive(Serialize)]
	pub struct RoundState<H, N> {
		pub prevote_ghost: Option<(H, N)>,
		pub finalized: Option<(H, N)>,
		pub estimate: Option<(H, N)>,
		pub completable: bool,
	}

	impl<H, N> From<crfg::CrfgState<H, N>> for CrfgState<H, N> where
		H: Clone,
		N: Clone,
	{
		fn from(t: crfg::CrfgState<H, N>) -> CrfgState<H, N> {
			CrfgState {
				config: t.config.into(),
				set_id: t.set_id,
				voters: t.voters.into(),
				set_status: t.set_status.into(),
			}
		}
	}

	impl From<crfg::Config> for Config {
		fn from(t: crfg::Config) -> Config {
			Config {
				gossip_duration: t.gossip_duration,
				justification_period: t.justification_period,
				local_key_public: t.local_key.map(|x| Public(x.public().0.to_vec())),
				local_next_key_public: t.local_next_key.map(|x| Public(x.public().0.to_vec())),
				name: t.name,
			}
		}
	}

	impl From<Arc<grandpa::VoterSet<AuthorityId>>> for VoterSet {
		fn from(t: Arc<grandpa::VoterSet<AuthorityId>>) -> VoterSet {
			VoterSet {
				weights: t.weights.iter().map(|(k, v)| (Public(k.0.to_vec()), v.clone().into())).collect(),
				voters: t.voters.iter().map(|(l, r)| (Public(l.0.to_vec()), *r)).collect(),
				threshold: t.threshold,
			}
		}
	}

	impl From<grandpa::VoterInfo> for VoterInfo {
		fn from(t: grandpa::VoterInfo) -> VoterInfo {
			VoterInfo {
				canon_idx: t.canon_idx(),
				weight: t.weight(),
			}
		}
	}

	impl<H, N> From<crfg::VoterSetState<H, N>> for VoterSetState<H, N> {
		fn from(t: crfg::VoterSetState<H, N>) -> VoterSetState<H, N> {
			match t {
				crfg::VoterSetState::Paused(id, round_state) => VoterSetState::Paused(id, round_state.into()),
				crfg::VoterSetState::Live(id, round_state) => VoterSetState::Live(id, round_state.into()),
			}
		}
	}

	impl<H, N> From<grandpa::State<H, N>> for RoundState<H, N> {
		fn from(t: grandpa::State<H, N>) -> RoundState<H, N> {
			RoundState {
				prevote_ghost: t.prevote_ghost,
				finalized: t.finalized,
				estimate: t.estimate,
				completable: t.completable,
			}
		}
	}
}


