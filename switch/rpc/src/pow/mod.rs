
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

#[rpc]
pub trait PowApi<Hash> {
    #[rpc(name = "getwork")]
    fn get_job(&self) -> errors::Result<Work<Hash>>;

    #[rpc(name = "submit_job")]
    fn submit_job(&self, work: Work<Hash>) -> errors::Result<SubmitResponse>;
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SubmitResponse {

}

pub struct PowWork<Hashing> where
    Hashing: HashT + Send + Sync + 'static,
    Hashing::Output: Ord + Encode + Decode + Send + Sync + 'static,
{
    work_manager: Arc<dyn WorkManager<Hashing=Hashing>>
}

pub fn create_worker_manager(config: Config) -> PowWork<BlakeTwo256>
{
    let mut work_manager = crate::work_manager::DefaultWorkManager::<
        yee_runtime::BlockNumber,
        yee_runtime::AuthorityId,
        BlakeTwo256>::new(config);
    if let Err(e) = work_manager.start() {
        // todo
    }
    let wm = Arc::new(work_manager);
    PowWork::new(wm)
}

impl<Hashing> PowWork<Hashing> where
    Hashing: HashT + Send + Sync + 'static,
    Hashing::Output: Ord + Encode + Decode + Send + Sync + 'static,
{
    pub fn new(wm: Arc<dyn WorkManager<Hashing=Hashing>>) -> Self {
        Self {
            work_manager: wm,
        }
    }
}

impl<Hashing, Hash> PowApi<Hash> for PowWork<Hashing> where
    Hashing: HashT + Send + Sync + 'static,
    Hashing::Output: Ord + Encode + Decode + Send + Sync + 'static,
    Hashing::Output: Hash
{
    fn get_job(&self) -> errors::Result<mining::work_manager::Work<Hash>> {
        self.work_manager.get_work()
    }

    fn submit_job(&self, work: Work<Hashing::Output>) -> errors::Result<SubmitResponse> {

        Ok(SubmitResponse{})
    }
}