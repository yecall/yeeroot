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
    inherents::{
        InherentDataProviders, RuntimeString,
    },
    parity_codec::{
        Decode, Encode,
    },
    primitives::Blake2Hasher,
    runtime_primitives::{
        generic::{
            BlockId, DigestItem,
        },
        traits::{
            Block, Header, Digest as DigestT, DigestFor, DigestItem as DigestItemT, DigestItemFor,
            Hash as HashT,
            ProvideRuntimeApi,
            One,
        },
    },
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

/// Generated module index in construct_runtime!
/// module specific log entries are prefixed by it and
///
/// MUST MATCH WITH construct_runtime MACRO ORDER
///
pub const GENERATED_MODULE_LOG_PREFIX: u8 = 2;

pub trait ShardingDigestItem<N>: Sized {
    fn sharding_info(num: N, cnt: N) -> Self;
    fn as_sharding_info(&self) -> Option<(N, N)>;
}

impl<N, Hash, AuthorityId, SealSignature> ShardingDigestItem<N> for DigestItem<Hash, AuthorityId, SealSignature> where
    N: Decode + Encode,
{
    fn sharding_info(num: N, cnt: N) -> Self {
        let prefix: [u8; 2] = [GENERATED_MODULE_LOG_PREFIX, 0];
        let data = Encode::encode(&(prefix, num, cnt));
        DigestItem::Other(data)
    }

    fn as_sharding_info(&self) -> Option<(N, N)> {
        match self {
            DigestItem::Other(data) if data.len() >= 4
                && data[0] == GENERATED_MODULE_LOG_PREFIX
                && data[1] == 0 => {
                let input = &mut &data[2..];
                let num = Decode::decode(input)?;
                let cnt = Decode::decode(input)?;
                Some((num, cnt))
            }
            _ => None
        }
    }
}

pub fn prepare_sharding<F, C, B, AuthorityId, SealSignature>(
    node_config: &NodeConfig,
    client: Arc<C>,
    backend: Arc<B>,
) -> Result<(), substrate_service::Error> where
    F: ServiceFactory,
    FactoryBlock<F>: Block,
    <FactoryBlock<F> as Block>::Header: Header,
    DigestFor<FactoryBlock<F>>: DigestT<Item=DigestItem<<FactoryBlock<F> as Block>::Hash, AuthorityId, SealSignature>>,
    DigestItemFor<FactoryBlock<F>>: DigestItemT<Hash=<FactoryBlock<F> as Block>::Hash> + ShardingDigestItem<u32>,
    C: ProvideRuntimeApi + ChainHead<FactoryBlock<F>>,
    <C as ProvideRuntimeApi>::Api: ShardingAPI<FactoryBlock<F>>,
    B: ClientBackend<FactoryBlock<F>, Blake2Hasher>,
    <B as ClientBackend<FactoryBlock<F>, Blake2Hasher>>::BlockImportOperation: BlockImportOperation<FactoryBlock<F>, Blake2Hasher>,
    <<B as ClientBackend<FactoryBlock<F>, Blake2Hasher>>::BlockImportOperation as BlockImportOperation<FactoryBlock<F>, Blake2Hasher>>::State: StateBackend<Blake2Hasher>,
{
    let api = client.runtime_api();
    let last_block_header = client.best_block_header()?;
    let last_block_id = BlockId::hash(last_block_header.hash());

    let target_shard_num = node_config.shard_num as u32;

    let shard_info = last_block_header.digest().logs().iter().rev()
        .filter_map(ShardingDigestItem::as_sharding_info)
        .next();

    let (curr_shard, curr_count) = match shard_info {
        Some(shard_info) => {
            let (curr_shard, cnt) = shard_info;
            info!("chain data in shard {} / {}", curr_shard, cnt);
            if curr_shard != target_shard_num {
                let msg = format!("in chain shard {} while configured {}", curr_shard, target_shard_num);
                return Err(substrate_service::ErrorKind::Msg(msg).into());
            }

            (curr_shard, cnt)
        }
        None => {
            info!("build sharding block");
            let genesis_shard_cnt = api.get_genesis_shard_count(&last_block_id)?;
            let shard_digest = DigestItem::sharding_info(target_shard_num, genesis_shard_cnt);
            info!("shard info {} / {}", target_shard_num, genesis_shard_cnt);

            let extrinsics_root = <<<FactoryBlock<F> as Block>::Header as Header>::Hashing as HashT>::trie_root(::std::iter::empty::<(&[u8], &[u8])>());
            let mut header = <<FactoryBlock<F> as Block>::Header as Header>::new(
                last_block_header.number().to_owned() + One::one(),
                extrinsics_root,
                *last_block_header.state_root(),
                last_block_header.hash(),
                Default::default(),
            );
            header.digest_mut().push(shard_digest);

            let mut op = backend.begin_operation()?;
            backend.begin_state_operation(&mut op, last_block_id)?;
            op.set_block_data(header, Some(vec![]), None, NewBlockState::Final)?;
            backend.commit_operation(op)?;

            (target_shard_num, genesis_shard_cnt)
        }
    };

    register_inherent_data_provider(&node_config.inherent_data_providers, curr_shard, curr_count)?;

    Ok(())
}

fn register_inherent_data_provider(
    inherent_data_providers: &InherentDataProviders,
    shard_num: u32, shard_cnt: u32,
) -> Result<(), consensus_common::Error> {
    consensus::register_inherent_data_provider(inherent_data_providers)?;
    if !inherent_data_providers.has_provider(&srml_sharding::INHERENT_IDENTIFIER) {
        inherent_data_providers.register_provider(srml_sharding::InherentDataProvider::new(shard_num, shard_cnt))
            .map_err(inherent_to_common_error)
    } else {
        Ok(())
    }
}

fn inherent_to_common_error(err: RuntimeString) -> consensus_common::Error {
    consensus_common::ErrorKind::InherentData(err.into()).into()
}
