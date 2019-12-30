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

#![allow(unused_imports)]
pub mod misc;
pub mod mining;
mod errors;
use substrate_service::{Components, ComponentClient, ComponentBlock, ComponentExHash, RpcHandlerConstructor, FactoryFullConfiguration,
                        ServiceFactory, DefaultRpcHandlerConstructor};
use tokio::runtime::TaskExecutor;
use std::sync::Arc;
use substrate_rpc_services::{self, apis::system::SystemInfo};
use runtime_primitives::{
    BuildStorage, traits::{Block as BlockT, Header as HeaderT, ProvideRuntimeApi}, generic::BlockId
};
use client::{self, Client, runtime_api};
use network::{self, OnDemand};
use transaction_pool::txpool::{self, Options as TransactionPoolOptions, Pool as TransactionPool};
use std::marker::PhantomData;
use crate::mining::{Mining, MiningApi};
use crate::misc::{MiscApi, Misc};
use parking_lot::RwLock;
use yee_consensus_pow::{JobManager, DefaultJob};
use yee_runtime::opaque::{Block};
use substrate_primitives::{ed25519::Pair, Pair as PairT};
use parity_codec::{Decode, Encode, Codec};
use serde::de::Unexpected::Other;

pub struct FullRpcHandlerConstructor;

pub type LightRpcHandlerConstructor = DefaultRpcHandlerConstructor;

pub trait ProvideJobManager<J> {
    fn provide_job_manager(&self) -> Arc<RwLock<Option<Arc<dyn JobManager<Job=J>>>>>;
}

#[derive(Default, Clone)]
pub struct FullRpcExtra<J> {
    job_manager: Arc<RwLock<Option<Arc<dyn JobManager<Job=J>>>>>,
}

impl<C: Components> RpcHandlerConstructor<C> for FullRpcHandlerConstructor where
    ComponentClient<C>: ProvideRuntimeApi,
    <ComponentClient<C> as ProvideRuntimeApi>::Api: runtime_api::Metadata<ComponentBlock<C>>,
    <C::Factory as ServiceFactory>::Configuration: ProvideJobManager<DefaultJob<Block, <Pair as PairT>::Public>>,
{

    type RpcExtra = FullRpcExtra<DefaultJob<Block, <Pair as PairT>::Public>>;

    fn build_rpc_extra(config: &FactoryFullConfiguration<C::Factory>) -> Self::RpcExtra{
        FullRpcExtra{
            job_manager: config.custom.provide_job_manager(),
        }
    }

    fn new_rpc_handler(
        client: Arc<ComponentClient<C>>,
        network: Arc<dyn network::SyncProvider<ComponentBlock<C>>>,
        should_have_peers: bool,
        rpc_system_info: SystemInfo,
        task_executor: TaskExecutor,
        transaction_pool: Arc<TransactionPool<C::TransactionPoolApi>>,
        extra: Self::RpcExtra,
    ) -> substrate_rpc_services::RpcHandler{
        let client = client.clone();
        let subscriptions = substrate_rpc_services::apis::Subscriptions::new(task_executor);
        let chain = substrate_rpc_services::apis::chain::Chain::new(client.clone(), subscriptions.clone());
        let state = substrate_rpc_services::apis::state::State::new(client.clone(), subscriptions.clone());
        let author = substrate_rpc_services::apis::author::Author::new(
            client.clone(), transaction_pool.clone(), subscriptions
        );
        let system = substrate_rpc_services::apis::system::System::new(
            rpc_system_info.clone(), network.clone(), should_have_peers
        );

        let mut io = substrate_rpc_services::rpc_handler::<ComponentBlock<C>, ComponentExHash<C>, _, _, _, _>(
            state,
            chain,
            author,
            system,
        );

        let mining = Mining::new(extra.job_manager);
        io.extend_with(mining.to_delegate());

        let misc = Misc::<<C::Factory as ServiceFactory>::Block>::new();
        io.extend_with(misc.to_delegate());

        io
    }
}
