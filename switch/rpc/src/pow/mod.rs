use jsonrpc_derive::rpc;
use primitives::{sr25519, storage::{StorageKey, StorageData}};
use crate::rpc::futures::{Future, Stream};
use crate::Config;
use crate::client::RpcClient;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use parity_codec::{KeyedVec, Codec, Decode, Encode, Input, Compact};
use sr_io::blake2_256;
use num_bigint::BigUint;
use yee_runtime::AccountId;
use yee_sharding_primitives::utils::shard_num_for_bytes;
use crate::errors;
use jsonrpc_core::{BoxFuture, Error, ErrorCode};
use crate::rpc::{self, futures::future::{self, FutureResult}};
use yee_serde_hex::Hex;
use yee_primitives::{Address, AddressCodec, Hrp};
use serde_json::Value;
use hex;
use yee_sr_primitives::SHARD_CODE_SIZE;
use serde_json::map::Entry::Vacant;
use serde::export::PhantomData;
use crate::work_manager::{WorkManager, Work};
use runtime_primitives::traits::{BlakeTwo256, Hash as HashT};
use std::sync::Arc;
use yee_serde_hex::SerdeHex;
use parking_lot::RwLock;
use yee_consensus_pow_primitives::PowTarget;
use tokio::timer::Interval;
use tokio::runtime::{Runtime, TaskExecutor};
use std::time::{Instant, Duration};
use log::warn;

#[rpc]
pub trait PowApi<Hash> {
    #[rpc(name = "get_work")]
    fn get_work(&self) -> errors::Result<Job<Hash>>;

    #[rpc(name = "submit_work")]
    fn submit_work(&self, work: SubmitJob<Hash>) -> errors::Result<SubmitResponse>;
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Job<Hash> {
    pub merkle_root: Hash,
    pub extra_data: Vec<u8>,
    pub target: PowTarget,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SubmitJob<Hash> {
    pub merkle_root: Hash,
    pub extra_data: Vec<u8>,
    pub nonce: u64,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SubmitResponse {
    pub received: u8,
}

pub struct Pow<WM: WorkManager> where {
    work_manager: Arc<RwLock<WM>>
}

impl<WM: WorkManager> Pow<WM> where
{
    pub fn new(wm: Arc<RwLock<WM>>) -> Self {
        Self {
            work_manager: wm,
        }
    }
}

impl<WM> PowApi<<WM::Hashing as HashT>::Output> for Pow<WM> where
    WM: WorkManager + Send + Sync + 'static,
    <WM::Hashing as HashT>::Output: Decode + Encode,
{
    fn get_work(&self) -> errors::Result<Job<<WM::Hashing as HashT>::Output>> {
        let work = self.work_manager.read().get_work()?;
        Ok(Job { merkle_root: work.merkle_root, extra_data: work.extra_data, target: work.target })
    }

    fn submit_work(&self, job: SubmitJob<<WM::Hashing as HashT>::Output>) -> errors::Result<SubmitResponse> {
        let work = match self.work_manager.read().get_work_by_merkle(job.merkle_root) {
            Ok(work) => work,
            Err(e) => return Err(errors::ErrorKind::SumbitJobError.into())
        };
        if self.work_manager.write().submit_work(work).is_err() {
            // todo
        }
        Ok(SubmitResponse { received: 1u8 })
    }
}