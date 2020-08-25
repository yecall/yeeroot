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

pub mod primitives;

use substrate_service::{ServiceFactory, ComponentClient, FullComponents, Components};
use yee_consensus_pow::{JobManager, DefaultJob, PowSeal,
                        WorkProof as DefaultWorkProof, ProofNonce as DefaultProofNonce, ProofMulti as DefaultProofMulti};
use yee_consensus_pow_primitives::PowTarget;
use parking_lot::RwLock;
use yee_runtime::opaque::Block;
use std::sync::Arc;
use jsonrpc_derive::rpc;
use log::{info, warn};
use basic_authorship::ProposerFactory;
use jsonrpc_core::BoxFuture;
use jsonrpc_core::futures::future::{self, Future, IntoFuture};
use runtime_primitives::traits::{Block as BlockT, ProvideRuntimeApi, BlakeTwo256, Header};
use consensus_common::{Environment, Proposer};
use crate::errors;
use client::{ChainHead, blockchain::HeaderBackend};
use tokio::timer::Interval;
use std::time::{Instant, Duration};
use parity_codec::alloc::collections::HashMap;
use parity_codec::{Decode, Encode};
use std::ops::Add;
use self::primitives::{Job, WorkProof, ProofNonce, ProofMulti, JobResult};
use std::fmt::Debug;
use serde::de::DeserializeOwned;
use yee_serde_hex::SerdeHex;
use lru::LruCache;
use log::debug;

const JOB_CACHE_SIZE: u32 = 60;

#[rpc]
pub trait MiningApi<Hash, Number, AuthorityId> where
    Hash: Encode,
    Number: Encode + SerdeHex,
    AuthorityId: Decode + Encode + Clone
{
    #[rpc(name = "mining_getJob")]
    fn get_job(&self) -> BoxFuture<Job<Hash, Number, AuthorityId>>;

    #[rpc(name = "mining_submitJob")]
    fn submit_job(&self, job_result: JobResult<Hash>) -> BoxFuture<Hash>;
}

pub struct MiningConfig {
    pub job_cache_size: Option<u32>,
}

pub struct Mining<B, AuthorityId> where
    B: BlockT,
    AuthorityId: Decode + Encode + Clone,
{
    job_manager: Arc<RwLock<Option<Arc<dyn JobManager<Job=DefaultJob<B, AuthorityId>>>>>>,
    cache: Arc<RwLock<LruCache<B::Hash, DefaultJob<B, AuthorityId>>>>,
}

impl<B, AuthorityId> Mining<B, AuthorityId> where
    B: BlockT,
    AuthorityId: Decode + Encode + Clone
{
    pub fn new(job_manager: Arc<RwLock<Option<Arc<dyn JobManager<Job=DefaultJob<B, AuthorityId>>>>>>, config: MiningConfig) -> Self {
        let cache_size = config.job_cache_size.unwrap_or(JOB_CACHE_SIZE) as usize;
        debug!("Job cache size: {}", cache_size);
        Self {
            job_manager,
            cache: Arc::new(RwLock::new(LruCache::new(cache_size))),
        }
    }

    fn put_cache(cache: Arc<RwLock<LruCache<B::Hash, DefaultJob<B, AuthorityId>>>>, job: DefaultJob<B, AuthorityId>) {
        cache.write().put(job.hash.clone(), job);
    }

    fn get_cache(cache: Arc<RwLock<LruCache<B::Hash, DefaultJob<B, AuthorityId>>>>, hash: &B::Hash) -> Option<DefaultJob<B, AuthorityId>> {

        cache.write().get(hash).cloned()
    }
}

impl<B, AuthorityId> MiningApi<B::Hash, <B::Header as Header>::Number, AuthorityId> for Mining<B, AuthorityId> where
    B: BlockT,
    AuthorityId: Decode + Encode + Clone + Send + Sync + Debug + 'static,
    <B::Header as Header>::Number: SerdeHex,
{
    fn get_job(&self) -> BoxFuture<Job<B::Hash, <B::Header as Header>::Number, AuthorityId>> {
        let job_manager = match self.job_manager.read().as_ref() {
            Some(j) => j.to_owned(),
            None => return Box::new(future::err(errors::Error::from(errors::ErrorKind::NotReady).into())),
        };

        let cache = self.cache.clone();

        Box::new(job_manager.get_job().into_future().map(move |job| {
            Self::put_cache(cache, job.clone());
            job.into()
        }).map_err(parse_error).map_err(|e| e.into()))
    }

    fn submit_job(&self, job_result: JobResult<B::Hash>) -> BoxFuture<B::Hash>{

        log::debug!("job_result: {:?}", job_result);

        let job_manager = match self.job_manager.read().as_ref() {
            Some(j) => j.to_owned(),
            None => return Box::new(future::err(errors::Error::from(errors::ErrorKind::NotReady).into())),
        };

        let cache = self.cache.clone();

        let mut job = match Self::get_cache(cache, &job_result.hash) {
            Some(job) => job,
            None => return Box::new(future::err(errors::Error::from(errors::ErrorKind::JobNotFound).into())),
        };

        job.digest_item.work_proof = job_result.digest_item.work_proof.into();

        Box::new(job_manager.submit_job(job).map_err(parse_error).map_err(
            |e| {
                warn!("submit job error: {:?}", e);
                e
            }
        ).map_err(|e| e.into()))

    }
}


fn parse_error<E: Into<errors::Error>>(error: E) -> errors::Error {
    error.into()
}

