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
use yee_consensus_pow_primitives::DifficultyType;
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

const JOB_LIFE: Duration = Duration::from_secs(300);

#[rpc]
pub trait MiningApi<Hash, Header, AuthorityId> where
    Hash: Encode,
    Header: Encode,
    AuthorityId: Decode + Encode + Clone
{
    #[rpc(name = "mining_getJob")]
    fn get_job(&self) -> BoxFuture<Job<Hash, Header, AuthorityId>>;

    #[rpc(name = "mining_submitJob")]
    fn submit_job(&self, job_result: JobResult<Hash, AuthorityId>) -> BoxFuture<Hash>;
}

pub struct Mining<B, AuthorityId> where
    B: BlockT,
    AuthorityId: Decode + Encode + Clone,
{
    job_manager: Arc<RwLock<Option<Arc<JobManager<Job=DefaultJob<B, AuthorityId>>>>>>,
    cache: Arc<RwLock<HashMap<B::Hash, (DefaultJob<B, AuthorityId>, Instant)>>>,
}

impl<B, AuthorityId> Mining<B, AuthorityId> where
    B: BlockT,
    AuthorityId: Decode + Encode + Clone
{
    pub fn new(job_manager: Arc<RwLock<Option<Arc<JobManager<Job=DefaultJob<B, AuthorityId>>>>>>) -> Self {
        Self {
            job_manager,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn put_cache(cache: Arc<RwLock<HashMap<B::Hash, (DefaultJob<B, AuthorityId>, Instant)>>>, job: DefaultJob<B, AuthorityId>) {
        let expire_at = Instant::now().add(JOB_LIFE);
        cache.write().insert(job.hash.clone(), (job.clone(), expire_at));
    }
}

impl<B, AuthorityId> MiningApi<B::Hash, B::Header, AuthorityId> for Mining<B, AuthorityId> where
    B: BlockT,
    AuthorityId: Decode + Encode + Clone + Send + Sync + Debug + 'static
{
    fn get_job(&self) -> BoxFuture<Job<B::Hash, B::Header, AuthorityId>> {
        let job_manager = match self.job_manager.read().as_ref() {
            Some(j) => j.to_owned(),
            None => return Box::new(future::err(errors::Error::from(errors::ErrorKind::NotReady).into())),
        };

        let cache = self.cache.clone();

        Box::new(job_manager.get_job().into_future().map(move |job| {
            Mining::put_cache(cache, job.clone());
            job.into()
        }).map_err(parse_error).map_err(|e| e.into()))
    }

    fn submit_job(&self, job_result: JobResult<B::Hash, AuthorityId>) -> BoxFuture<B::Hash>{

        log::info!("job_result: {:?}", job_result);

        Box::new(future::err(errors::Error::from(errors::ErrorKind::Unimplemented).into()))

    }
}


fn parse_error<E: Into<errors::Error>>(error: E) -> errors::Error {
    error.into()
}
