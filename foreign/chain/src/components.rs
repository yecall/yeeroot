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

use {
    exit_future::Signal,
    log::debug,
    std::{
        marker::PhantomData,
        ops::Deref,
        sync::Arc,
    },
    tokio::runtime::TaskExecutor,
};
use {
    inherents::pool::InherentsPool,
    keystore::Store as Keystore,
    rpc::apis::system::SystemInfo,
    substrate_service::{
        ComponentBlock, CodeExecutor, OnDemand, FactoryBlock, Components, LightBackend, LightExecutor, ServiceFactory, ComponentClient, ComponentExtrinsic, FactoryFullConfiguration, RpcHandlerConstructor,
    },
    transaction_pool::txpool::{
        Options as TransactionPoolOptions,
        Pool as TransactionPool,
    },
};

struct dummy{}

impl<C: Components> RpcHandlerConstructor<C> for dummy {
    fn new_rpc_handler(
        client: Arc<ComponentClient<C>>,
        network: Arc<network::SyncProvider<ComponentBlock<C>>>,
        should_have_peers: bool,
        rpc_system_info: SystemInfo,
        task_executor: TaskExecutor,
        transaction_pool: Arc<TransactionPool<C::TransactionPoolApi>>,
    ) -> rpc::RpcHandler {
        Default::default()
    }
}

pub struct ForeignComponents<Factory> where
    Factory: ServiceFactory + Send + Sync,
    <Factory as ServiceFactory>::Configuration: Send + Sync,
{
    _factory: PhantomData<Factory>,
    service: ForeignService<ForeignComponents<Factory>>,
}

impl<Factory> ForeignComponents<Factory> where
    Factory: ServiceFactory + Send + Sync,
    <Factory as ServiceFactory>::Configuration: Send + Sync,
{
    pub fn new(
        config: FactoryFullConfiguration<Factory>,
        task_executor: TaskExecutor,
    ) -> Result<Self, substrate_service::Error> {
        Ok(
            Self {
                _factory: Default::default(),
                service: ForeignService::new(config, task_executor)?,
            }
        )
    }
}

impl<Factory> Deref for ForeignComponents<Factory> where
    Factory: ServiceFactory + Send + Sync,
    <Factory as ServiceFactory>::Configuration: Send + Sync,
{
    type Target = ForeignService<Self>;

    fn deref(&self) -> &Self::Target {
        &self.service
    }
}

impl<Factory> Components for ForeignComponents<Factory> where
    Factory: ServiceFactory + Send + Sync,
    <Factory as ServiceFactory>::Configuration: Send + Sync,
{
    type Factory = Factory;
    type Backend = LightBackend<Factory>;
    type Executor = LightExecutor<Factory>;
    type RuntimeApi = Factory::RuntimeApi;
    type RuntimeServices = Self;
    type TransactionPoolApi = <Factory as ServiceFactory>::LightTransactionPoolApi;
    type ImportQueue = <Factory as ServiceFactory>::LightImportQueue;
    type RpcHandlerConstructor = dummy;

    fn build_client(
        config: &FactoryFullConfiguration<Factory>,
        executor: CodeExecutor<Self::Factory>,
    ) -> Result<(Arc<ComponentClient<Self>>, Option<Arc<OnDemand<FactoryBlock<Self::Factory>>>>),
        substrate_service::Error> {
        unimplemented!()
    }

    fn build_transaction_pool(
        config: TransactionPoolOptions, client: Arc<ComponentClient<Self>>,
    ) -> Result<TransactionPool<Self::TransactionPoolApi>, substrate_service::Error> {
        Factory::build_light_transaction_pool(config, client)
    }

    fn build_import_queue(
        config: &mut FactoryFullConfiguration<Self::Factory>,
        client: Arc<ComponentClient<Self>>,
    ) -> Result<Self::ImportQueue, substrate_service::Error> {
        Factory::build_light_import_queue(config, client)
    }
}

pub struct ForeignService<Components>
    where Components: substrate_service::Components {
    client: Arc<ComponentClient<Components>>,
    network: Option<Arc<substrate_service::NetworkService<Components::Factory>>>,
    transaction_pool: Arc<TransactionPool<Components::TransactionPoolApi>>,
    inherents_pool: Arc<InherentsPool<ComponentExtrinsic<Components>>>,
    keystore: Keystore,
    exit: ::exit_future::Exit,
    signal: Option<Signal>,
    /// Configuration of this Service
    pub config: FactoryFullConfiguration<Components::Factory>,
}

impl<Components> ForeignService<Components>
    where Components: substrate_service::Components {
    pub fn new(
        mut config: FactoryFullConfiguration<Components::Factory>,
        task_executor: TaskExecutor,
    ) -> Result<Self, substrate_service::Error> {
        unimplemented!()
    }
}

impl<Components> Drop for ForeignService<Components>
    where Components: substrate_service::Components {
    fn drop(&mut self) {
        debug!(target: "service", "Foreign service shutdown");

        drop(self.network.take());
        if let Some(signal) = self.signal.take() {
            signal.fire();
        }
    }
}
