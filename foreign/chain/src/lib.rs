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
    },
    log::info,
    tokio::runtime::TaskExecutor,
};
use {
    substrate_service::{
        ComponentExHash,
        FactoryFullConfiguration, NetworkProvider,
        LightComponents, ServiceFactory,
    },
};

pub trait ForeignChainConfig {
    fn get_shard_num(&self) -> u16;
    fn set_shard_num(&mut self, shard: u16);

    fn get_shard_count(&self) -> u16;
}

pub struct ForeignChain<F: ServiceFactory> {
    components: HashMap<u16, LightComponents<F>>,
}

impl<F> ForeignChain<F> where
    F: ServiceFactory,
    FactoryFullConfiguration<F>: Clone,
    <F as ServiceFactory>::Configuration: ForeignChainConfig,
{
    pub fn new(
        config: &FactoryFullConfiguration<F>,
        network_provider: impl NetworkProvider<F, ComponentExHash<LightComponents<F>>> + Clone,
        task_executor: TaskExecutor,
    ) -> Result<Self, substrate_service::Error> {
        let curr_shard = config.custom.get_shard_num();
        let shard_count = config.custom.get_shard_count();

        info!(
            "start Foreign chain for shard {} / {}",
            curr_shard, shard_count
        );

        let mut components = HashMap::new();
        for i in 0_u16..shard_count {
            if i == curr_shard {
                continue;
            }
            info!("create foreign chain {}", i);
            let mut shard_config = config.to_owned();
            shard_config.keystore_path = format!("{}-{}", config.keystore_path, i);
            shard_config.database_path = format!("{}-{}", config.database_path, i);
            shard_config.custom.set_shard_num(i);
            let network_id = i as u32;
            let shard_component = LightComponents::new_from_provider(
                shard_config, network_provider.clone(), network_id, task_executor.clone(),
            )?;
            components.insert(i, shard_component);
        }

        Ok(Self {
            components,
        })
    }

    pub fn get_shard_component(&self, shard: u16) -> Option<&LightComponents<F>> {
        self.components.get(&shard)
    }
}
