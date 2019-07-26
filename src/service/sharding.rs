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
    std::sync::Arc,
    log::info,
};
use {
    parity_codec::Encode,
    consensus::register_inherent_data_provider,
    primitives::Blake2Hasher,
    runtime_primitives::{
        generic::BlockId,
        traits::{
            Block, Header,
            Hash as HashT,
            ProvideRuntimeApi,
            One,
        },
    },
    sr_io::twox_128,
    state_machine::backend::Backend as StateBackend,
    substrate_client::{
        ChainHead,
        backend::{
            Backend as ClientBackend,
            BlockImportOperation,
            NewBlockState,
        },
    },
    substrate_service::{ServiceFactory, FactoryBlock},
};
use {
    sharding_primitives::ShardingAPI,
};
use super::NodeConfig;

pub fn prepare_sharding<F, C, B>(
    node_config: &NodeConfig,
    client: Arc<C>,
    backend: Arc<B>,
) -> Result<(), substrate_service::Error> where
    F: ServiceFactory,
    <FactoryBlock<F> as Block>::Header: Header,
    C: ProvideRuntimeApi + ChainHead<FactoryBlock<F>>,
    <C as ProvideRuntimeApi>::Api: ShardingAPI<FactoryBlock<F>>,
    B: ClientBackend<FactoryBlock<F>, Blake2Hasher>,
    <B as ClientBackend<FactoryBlock<F>, Blake2Hasher>>::BlockImportOperation: BlockImportOperation<FactoryBlock<F>, Blake2Hasher>,
    <<B as ClientBackend<FactoryBlock<F>, Blake2Hasher>>::BlockImportOperation as BlockImportOperation<FactoryBlock<F>, Blake2Hasher>>::State: StateBackend<Blake2Hasher>,
{
    register_inherent_data_provider(&node_config.inherent_data_providers)?;

    let api = client.runtime_api();
    let last_block_header = client.best_block_header()?;
    let last_block_id = BlockId::hash(last_block_header.hash());

    let shard_count = api.get_shard_count(&last_block_id)?;
    let target_shard_num = node_config.shard_num as u32;
    info!("sharding info: {} in {}", target_shard_num, shard_count);

    if let Some(curr_shard) = api.get_curr_shard(&last_block_id)? {
        info!("chain data in shard {}", curr_shard);
        if curr_shard != target_shard_num {
            let msg = format!("in chain shard {} while configured {}", curr_shard, target_shard_num);
            return Err(substrate_service::ErrorKind::Msg(msg).into());
        }
    } else {
        info!("build sharding block");
        let storage_key = twox_128("Sharding CurrentShard".as_bytes()).to_vec();
        let updates = vec![(storage_key, Some(Encode::encode(&target_shard_num)))];

        let mut op = backend.begin_operation()?;
        backend.begin_state_operation(&mut op, last_block_id)?;

        let (root, overlay) = op.state()
            .expect("qed")
            .expect("not for light")
            .storage_root(updates.iter().cloned());
        op.update_db_storage(overlay)?;
        let state_root = root.into();

        let extrinsics_root = <<<FactoryBlock<F> as Block>::Header as Header>::Hashing as HashT>::trie_root(::std::iter::empty::<(&[u8], &[u8])>());
        let header = <<FactoryBlock<F> as Block>::Header as Header>::new(
            last_block_header.number().to_owned() + One::one(),
            extrinsics_root,
            state_root,
            last_block_header.hash(),
            Default::default(),
        );
        op.set_block_data(header, Some(vec![]), None, NewBlockState::Final)?;
        backend.commit_operation(op)?;
    }

    Ok(())
}
