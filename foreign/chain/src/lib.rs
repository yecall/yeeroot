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
    std::{
        collections::HashMap,
        marker::PhantomData,
        sync::Arc,
    },
    log::info,
    tokio::runtime::TaskExecutor,
};
use {
    runtime_primitives::{
        generic::BlockId,
        traits::{
            Block, Header,
            ProvideRuntimeApi,
        },
    },
    substrate_client::ChainHead,
    substrate_service::{
        FactoryFullConfiguration, NetworkProvider,
        FactoryBlock, LightComponents, ServiceFactory,
    },
};
use {
    sharding_primitives::ShardingAPI,
};

pub trait ForeignChainConfig {
    fn get_shard_num(&self) -> u32;
    fn set_shard_num(&mut self, shard: u32);
}

pub struct ForeignChain<F: ServiceFactory, C> {
    _phantom: PhantomData<(F, C)>,
    components: HashMap<u32, LightComponents<F>>,
}

impl<F, C> ForeignChain<F, C> where
    F: ServiceFactory,
    <FactoryBlock<F> as Block>::Header: Header,
    FactoryFullConfiguration<F>: Clone,
    <F as ServiceFactory>::Configuration: ForeignChainConfig,
    C: ProvideRuntimeApi + ChainHead<FactoryBlock<F>>,
    <C as ProvideRuntimeApi>::Api: ShardingAPI<FactoryBlock<F>>,
{
    pub fn new(
        config: &FactoryFullConfiguration<F>,
        network_provider: impl NetworkProvider<LightComponents<F>> + Clone,
        client: Arc<C>,
        task_executor: TaskExecutor,
    ) -> Result<Self, substrate_service::Error> {
        let api = client.runtime_api();
        let last_block_header = client.best_block_header()?;
        let last_block_id = BlockId::hash(last_block_header.hash());
        let curr_shard = config.custom.get_shard_num();
        let shard_count = api.get_shard_count(&last_block_id)?;

        info!(
            "start Foreign chain for shard {} / {}",
            curr_shard, shard_count
        );

        let mut components = HashMap::new();
        for i in 0_u32..shard_count {
            if i == curr_shard {
                continue;
            }
            info!("create foreign chain {}", i);
            let mut shard_config = config.to_owned();
            shard_config.keystore_path = format!("{}-{}", config.keystore_path, i);
            shard_config.database_path = format!("{}-{}", config.database_path, i);
            shard_config.custom.set_shard_num(i);
            let shard_component = LightComponents::new_foreign(
                shard_config, network_provider.clone(), i, task_executor.clone(),
            )?;
            components.insert(i, shard_component);
        }

        Ok(Self {
            _phantom: Default::default(),
            components,
        })
    }

    pub fn get_shard_component(&self, shard: u32) -> Option<&LightComponents<F>> {
        self.components.get(&shard)
    }
}
