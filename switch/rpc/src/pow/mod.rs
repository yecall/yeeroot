
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
use crate::work_manager::{DefaultWorkManager, Work};
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

pub struct PowWork<Number: SerdeHex, AuthorityId, Hashing> {
    work_manager: DefaultWorkManager<Number, AuthorityId, Hashing>
}

pub fn create_worker_manager<Number: SerdeHex, AuthorityId, Hashing>(config: Config) -> PowWork<Number, AuthorityId, Hashing>
{
    let mut work_manager = crate::work_manager::DefaultWorkManager::<
        yee_runtime::BlockNumber,
        yee_runtime::AuthorityId,
        BlakeTwo256>::new(config);
    if let Err(e) = work_manager.start() {
        // todo
    }
    //let wm = Arc::new(work_manager);
    PowWork::new(work_manager)
}

impl<Number: SerdeHex, AuthorityId, Hashing> PowWork<Number, AuthorityId, Hashing> where
    <Hashing as HashT>::Output: Decode + Encode,
{
    pub fn new(work_manager: DefaultWorkManager<Number, AuthorityId, Hashing>) -> Self {
        Self {
            work_manager
        }
    }
}

impl<Number: SerdeHex, AuthorityId, Hashing> PowApi<<Hashing as HashT>::Output> for PowWork<Number, AuthorityId, Hashing> where
    <Hashing as HashT>::Output: Decode + Encode,
{
    fn get_job(&self) -> errors::Result<mining::work_manager::Work<<Hashing as HashT>::Output>> {
        self.work_manager.get_work()
    }

    fn submit_job(&self, work: Work<Hash>) -> errors::Result<SubmitResponse> {

        Ok(SubmitResponse{})
    }
}