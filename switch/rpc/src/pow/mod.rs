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
use log::{info, warn};
use std::ptr::read_unaligned;

#[rpc]
pub trait PowApi<Hash> {
    #[rpc(name = "get_work")]
    fn get_work(&self) -> errors::Result<Job<Hash>>;

    #[rpc(name = "submit_work")]
    fn submit_work(&self, data: String) -> errors::Result<SubmitResponse>;
    // fn submit_work(&self, work: SubmitJob<Hash>) -> errors::Result<SubmitResponse>;
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
    pub reject_reason: Vec<u8>,
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
        info!("get_work: {:?}", work);

        // for test
        let root = work.merkle_root;
        if let Ok(_work) = self.work_manager.read().get_work_by_merkle(root){
            info!("{}", "ok");
        }

        Ok(Job { merkle_root: work.merkle_root, extra_data: work.extra_data, target: work.target })
    }

    // fn submit_work(&self, job: SubmitJob<<WM::Hashing as HashT>::Output>) -> errors::Result<SubmitResponse> {
    //     let work = match self.work_manager.read().get_work_by_merkle(job.merkle_root) {
    //         Ok(work) => work,
    //         Err(e) => return Err(errors::ErrorKind::SumbitJobError.into())
    //     };
    //     if self.work_manager.write().submit_work(work).is_err() {
    //         // todo
    //     }
    //     Ok(SubmitResponse { received: 1u8 })
    // }

    fn submit_work(&self, data: String) -> errors::Result<SubmitResponse> {
        warn!("submint work: {}", data);
        if let Ok(bytes) = hex::decode(data) {
            let bytes = bytes.as_slice();
            let mut data = &bytes[..32];
            let merkle_root = Decode::decode(&mut data).unwrap();
            let mut work = match self.work_manager.read().get_work_by_merkle(merkle_root) {
                Ok(work) => work,
                Err(e) => return Err(errors::ErrorKind::SumbitJobError.into())
            };

            warn!("target: {:?}", target_to_hex(work.target.clone()));
            work.extra_data = bytes[33..72].to_vec();

            let nonce = &bytes[72..80];
            let mut buf = [0u8;8];
            for i in 0..8 {
                buf[i] = nonce[7-i];
            }
            let mut non=0u64;
            if let Ok(nonce) = u64::from_bytes(&buf[..]){
                work.nonce = Some(nonce);
                non=nonce;
            }

            let source = (work.merkle_root.clone(), work.extra_data.clone(), non);
            let buf = source.encode();
            warn!("submit work,data: {:?}", hex::encode(buf));
            let source_hash = WM::Hashing::hash_of(&source);
            let nonce_target = PowTarget::from(source_hash.as_ref());
            let n_t = target_to_hex(nonce_target.clone());
            work.nonce_target = Some(nonce_target);
            warn!("submit work, total work: merkle:{:?}, nonce-target:{:?}, nonce:{:?}", work.merkle_root.clone(), n_t, work.nonce);
            if self.work_manager.write().submit_work(work).is_err() {
                warn!("submit a invalid work.");
            }
        }
        Ok(SubmitResponse{reject_reason: vec![]})
    }
}

fn target_to_hex(input: PowTarget) -> String {
    let buf: Vec<u8> = input.0
        .iter()
        .map(|x| x.to_le_bytes().to_vec())
        .flatten()
        .collect();
    let mut buf = buf.as_slice();
    hex::encode(buf)
}

fn le_to_be(input: Vec<u8>) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::with_capacity(input.len());
    let size = input.len() / 4;
    for i in 0..size {
       result.append(&mut vec![input[i*4+3]]);
        result.append(&mut vec![input[i*4+2]]);
        result.append(&mut vec![input[i*4+1]]);
        result.append(&mut vec![input[i*4]]);
    }
    result
}