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

use substrate_service::{ServiceFactory, ComponentClient, FullComponents, Components};
use yee_consensus_pow::{JobManager, DefaultJob};
use parking_lot::RwLock;
use yee_runtime::opaque::Block;
use std::sync::Arc;
use jsonrpc_derive::rpc;
use log::info;
use basic_authorship::ProposerFactory;
use jsonrpc_core::BoxFuture;
use jsonrpc_core::futures::future::{self, Future, IntoFuture};
use runtime_primitives::traits::{Block as BlockT, ProvideRuntimeApi};
use consensus_common::{Environment, Proposer};
use crate::errors;
use client::{ChainHead, blockchain::HeaderBackend};

pub trait ProvideJobManager<J> {
    fn provide_job_manager(&self) -> Arc<RwLock<Option<Arc<JobManager<Job=J>>>>>;
}

#[rpc]
pub trait MiningApi<J> {
    #[rpc(name = "mining_getJob")]
    fn get_job(&self) -> BoxFuture<J>;
}

pub struct Mining<J> {
    job_manager: Arc<RwLock<Option<Arc<JobManager<Job=J>>>>>,
}

impl<J> Mining<J> {
    pub fn new(job_manager: Arc<RwLock<Option<Arc<JobManager<Job=J>>>>>) -> Self {
        Self {
            job_manager,
        }
    }
}

impl<J> MiningApi<J> for Mining<J> where
    J: Send + 'static
{
    fn get_job(&self) -> BoxFuture<J> {

        let job_manager = match self.job_manager.read().as_ref() {
            Some(j) => j.to_owned(),
            None => return Box::new(future::err(errors::Error::from(errors::ErrorKind::NotReady).into())),
        };

        Box::new(job_manager.get_job().into_future().map_err(parse_error).map_err(|e|e.into()))

    }
}


fn parse_error<E: Into<errors::Error>>(error: E) -> errors::Error{

    error.into()
}
