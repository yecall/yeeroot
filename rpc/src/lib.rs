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
use std::option::Option;
use std::default::Default;
use substrate_rpc_services::{self, apis::system::SystemInfo};
use runtime_primitives::{
    BuildStorage, traits::{Block as BlockT, Header as HeaderT, ProvideRuntimeApi, NumberFor}, generic::BlockId
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
use futures::sync::mpsc;
use yee_primitives::RecommitRelay;
use crfg::CrfgState;
use yee_foreign_network::SyncProvider;

pub struct FullRpcHandlerConstructor;

pub type LightRpcHandlerConstructor = DefaultRpcHandlerConstructor;

pub trait ProvideRpcExtra<J, B, H>
where B: BlockT
{
    fn provide_job_manager(&self) -> Arc<RwLock<Option<Arc<dyn JobManager<Job=J>>>>>;

    fn provide_recommit_relay_sender(&self) -> Arc<RwLock<Option<mpsc::UnboundedSender<RecommitRelay<B::Hash>>>>>;

    fn provide_crfg_state(&self) -> Arc<RwLock<Option<CrfgState<B::Hash, NumberFor<B>>>>>;

    fn provide_foreign_network(&self) -> Arc<RwLock<Option<Arc<dyn SyncProvider<B, H>>>>>;
}

#[derive(Clone)]
pub struct FullRpcExtra<J, B, H>
where B: BlockT {
    job_manager: Arc<RwLock<Option<Arc<dyn JobManager<Job=J>>>>>,
    recommit_relay_sender: Arc<RwLock<Option<mpsc::UnboundedSender<RecommitRelay<B::Hash>>>>>,
    crfg_state: Arc<RwLock<Option<CrfgState<B::Hash, NumberFor<B>>>>>,
    foreign_network: Arc<RwLock<Option<Arc<dyn SyncProvider<B, H>>>>>,
}

impl<C: Components> RpcHandlerConstructor<C> for FullRpcHandlerConstructor where
    ComponentClient<C>: ProvideRuntimeApi,
    <ComponentClient<C> as ProvideRuntimeApi>::Api: runtime_api::Metadata<ComponentBlock<C>>,
    <C::Factory as ServiceFactory>::Configuration: ProvideRpcExtra<
        DefaultJob<Block, <Pair as PairT>::Public>,
        <C::Factory as ServiceFactory>::Block,
        ComponentExHash<C>
    >,
{
    type RpcExtra = FullRpcExtra<
        DefaultJob<Block, <Pair as PairT>::Public>,
        <C::Factory as ServiceFactory>::Block,
        ComponentExHash<C>,
    >;

    fn build_rpc_extra(config: &FactoryFullConfiguration<C::Factory>) -> Self::RpcExtra{
        FullRpcExtra{
            job_manager: config.custom.provide_job_manager(),
            recommit_relay_sender: config.custom.provide_recommit_relay_sender(),
            crfg_state: config.custom.provide_crfg_state(),
            foreign_network: config.custom.provide_foreign_network(),
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

        let misc = Misc::new(
            extra.recommit_relay_sender.clone(),
            extra.crfg_state.clone(),
            transaction_pool.clone(),
            extra.foreign_network.clone(),
        );
        io.extend_with(misc.to_delegate());

        io
    }
}
