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

//! POW (Proof of Work) consensus in YeeChain

use {
    std::{fmt::Debug, marker::PhantomData, sync::Arc},
    futures::{Future, IntoFuture},
    log::{warn, info},
    parking_lot::RwLock,
};
use {
    client::{
        ChainHead,
        blockchain::HeaderBackend,
    },
    consensus_common::{
        BlockImport, Environment, Proposer, SyncOracle,
        import_queue::{
            BasicQueue,
            SharedBlockImport, SharedJustificationImport,
        },
    },
    inherents::{InherentDataProviders, RuntimeString},
    primitives::Pair,
    runtime_primitives::{
        codec::{
            Decode, Encode, Codec,
        },
        traits::{
            DigestItemFor,
            Block,
            ProvideRuntimeApi,
            Header,
        },
        generic::BlockId,
    },
};
use {
    pow_primitives::YeePOWApi,
};

pub use digest::CompatibleDigestItem;
pub use pow::{PowSeal, WorkProof, ProofNonce, ProofMulti,
              MiningAlgorithm, MiningHash, OriginalMerkleProof, CompactMerkleProof};
pub use job::{JobManager, DefaultJobManager, DefaultJob};
use yee_sharding::ShardingDigestItem;
use yee_sharding_primitives::{ShardingAPI, utils::shard_num_for};
use yee_srml_pow::RewardCondition;

mod job;
mod digest;
mod pow;
mod verifier;
mod worker;

pub struct Params<AccountId> {
    pub coinbase: AccountId,
    pub force_authoring: bool,
    pub mine: bool,
    pub shard_num: u16,
}

pub fn start_pow<B, P, C, I, E, AccountId, SO, OnExit>(
    local_key: Arc<P>,
    client: Arc<C>,
    block_import: Arc<I>,
    env: Arc<E>,
    sync_oracle: SO,
    on_exit: OnExit,
    inherent_data_providers: InherentDataProviders,
    job_manager: Arc<RwLock<Option<Arc<dyn JobManager<Job=DefaultJob<B, P::Public>>>>>>,
    params: Params<AccountId>,
) -> Result<impl Future<Item=(), Error=()>, consensus_common::Error> where
    B: Block,
    P: Pair + 'static,
    <P as Pair>::Public: Clone + Debug + Decode + Encode + Send + Sync,
    C: ChainHead<B> + HeaderBackend<B> + ProvideRuntimeApi + 'static,
    <C as ProvideRuntimeApi>::Api: YeePOWApi<B>,
    <C as ProvideRuntimeApi>::Api: ShardingAPI<B>,
    I: BlockImport<B, Error=consensus_common::Error> + Send + Sync + 'static,
    E: Environment<B> + Send + Sync + 'static,
    <E as Environment<B>>::Error: Debug + Send,
    <<<E as Environment<B>>::Proposer as Proposer<B>>::Create as IntoFuture>::Future: Send + 'static,
    AccountId: Clone + Debug + Decode + Encode + Default + Send + Sync + 'static,
    SO: SyncOracle + Send + Sync + Clone,
    OnExit: Future<Item=(), Error=()>,
    DigestItemFor<B>: CompatibleDigestItem<B, P::Public> + ShardingDigestItem<u16>,
{

    //validate coinbase
    let shard_count = get_shard_count::<B, _>(&client)?;
    let coinbase_shard_num = shard_num_for(&params.coinbase, shard_count).expect("qed");
    info!("Coinbase shard num: {}", coinbase_shard_num);
    if coinbase_shard_num != params.shard_num {
        panic!("Invalid coinbase: shard num is not accordant");
    }

    let inner_job_manager = Arc::new(DefaultJobManager::new(
        client.clone(),
        env.clone(),
        inherent_data_providers.clone(),
        local_key.public(),
        block_import.clone(),
    ));

    let mut reg_lock = job_manager.write();
    match *reg_lock {
        Some(_) => {
            warn!("job manager already registered");
            panic!("job manager can only be registered once");
        },
        None => {
            *reg_lock = Some(inner_job_manager.clone());
        }
    }

    let worker = Arc::new(worker::DefaultWorker::new(
        inner_job_manager.clone(),
        block_import,
        inherent_data_providers.clone(),
        params.coinbase,
    ));
    worker::start_worker(
        worker,
        sync_oracle,
        on_exit,
        params.mine)
}

/// POW chain import queue
pub type PowImportQueue<B> = BasicQueue<B>;

/// Start import queue for POW consensus
pub fn import_queue<B, C, AccountId, AuthorityId>(
    block_import: SharedBlockImport<B>,
    justification_import: Option<SharedJustificationImport<B>>,
    client: Arc<C>,
    inherent_data_providers: InherentDataProviders,
    coinbase: AccountId,
) -> Result<PowImportQueue<B>, consensus_common::Error> where
    B: Block,
    DigestItemFor<B>: CompatibleDigestItem<B, AuthorityId> + ShardingDigestItem<u16>,
    C: 'static + Send + Sync,
    AccountId: Codec + Send + Sync + 'static,
    AuthorityId: Decode + Encode + Clone + Send + Sync + 'static,
{
    register_inherent_data_provider(&inherent_data_providers, coinbase)?;

    let verifier = Arc::new(
        verifier::PowVerifier {
            client,
            inherent_data_providers,
            phantom: PhantomData,
        }
    );
    Ok(BasicQueue::new(verifier, block_import, justification_import))
}

pub fn register_inherent_data_provider<AccountId: 'static + Codec + Send + Sync>(
    inherent_data_providers: &InherentDataProviders,
    coinbase: AccountId,
) -> Result<(), consensus_common::Error> where
    AccountId : Codec + Send + Sync + 'static, {

    if !inherent_data_providers.has_provider(&yee_srml_pow::INHERENT_IDENTIFIER) {
        inherent_data_providers.register_provider(yee_srml_pow::InherentDataProvider::new(coinbase, RewardCondition::Normal))
            .map_err(inherent_to_common_error)
    } else {
        Ok(())
    }
}

fn inherent_to_common_error(err: RuntimeString) -> consensus_common::Error {
    consensus_common::ErrorKind::InherentData(err.into()).into()
}

fn get_shard_count<B, C>(client: &Arc<C>) -> consensus_common::error::Result<u16> where
    B: Block,
    C: ProvideRuntimeApi + ChainHead<B>,
    <C as ProvideRuntimeApi>::Api: ShardingAPI<B>, {
    let api = client.runtime_api();
    let last_block_header = client.best_block_header().map_err(|e| format!("{:?}", e))?;
    let last_block_id = BlockId::hash(last_block_header.hash());

    let shard_count = api.get_shard_count(&last_block_id).map(|x| x as u16).map_err(|e| format!("{:?}", e))?;

    Ok(shard_count)
}