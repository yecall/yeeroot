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
        FactoryBlock, FactoryFullConfiguration,
        LightComponents,
        ServiceFactory,
    },
};
use {
    sharding_primitives::ShardingAPI,
};

pub struct ForeignChain<F: ServiceFactory, C> {
    _phantom: PhantomData<(F, C)>,
    components: Vec<LightComponents<F>>,
}

impl<F, C> ForeignChain<F, C> where
    F: ServiceFactory,
    <FactoryBlock<F> as Block>::Header: Header,
    C: ProvideRuntimeApi + ChainHead<FactoryBlock<F>>,
    <C as ProvideRuntimeApi>::Api: ShardingAPI<FactoryBlock<F>>,
{
    pub fn new(
        config: FactoryFullConfiguration<F>,
        client: Arc<C>,
        task_executor: TaskExecutor,
    ) -> Result<Self, substrate_service::Error> {
        let api = client.runtime_api();
        let last_block_header = client.best_block_header()?;
        let last_block_id = BlockId::hash(last_block_header.hash());
        let curr_shard = api
            .get_curr_shard(&last_block_id)?
            .expect("shard info MUST be ready when foreign chain starts");
        let shard_count = api.get_shard_count(&last_block_id)?;

        info!(
            "start Foreign chain for shard {} / {}",
            curr_shard, shard_count
        );

        unimplemented!()
    }
}
