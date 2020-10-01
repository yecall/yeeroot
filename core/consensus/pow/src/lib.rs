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
    log::warn,
    parking_lot::RwLock,
};
use {
    client::{
        ChainHead,
        blockchain::HeaderBackend,
        BlockBody,
        BlockchainEvents,
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
            Block, Header,
            ProvideRuntimeApi,
            NumberFor,
        },
    },
    foreign_chain::{ForeignChain, ForeignChainConfig},
    yee_sharding_primitives::ShardingAPI,
};
use {
    pow_primitives::YeePOWApi,
};

pub use digest::CompatibleDigestItem;
pub use pow::{PowSeal, WorkProof, ProofNonce, ProofMulti,
              MiningAlgorithm, MiningHash, OriginalMerkleProof, CompactMerkleProof};
pub use job::{JobManager, DefaultJobManager, DefaultJob};
use yee_sharding::{ShardingDigestItem, ScaleOutPhaseDigestItem};
use yee_srml_pow::RewardCondition;
use yee_sharding_primitives::ScaleOut;
use primitives::H256;
use substrate_service::{ServiceFactory, FactoryFullConfiguration};
use yee_context::Context;

mod job;
mod digest;
mod pow;
mod verifier;
mod worker;
mod big_array;
mod extra_data;
pub mod fork;
// use big_array::BigArray;
pub use extra_data::ExtraData;

pub struct Params<AccountId, B> where
    B: Block,
{
    pub force_authoring: bool,
    pub mine: bool,
    pub shard_extra: ShardExtra<AccountId>,
    pub context: Context<B>,
    pub chain_spec_id: String,
}

pub fn start_pow<F, B, P, C, I, E, AccountId, SO, OnExit>(
    local_key: Arc<P>,
    client: Arc<C>,
    block_import: Arc<I>,
    env: Arc<E>,
    sync_oracle: SO,
    on_exit: OnExit,
    inherent_data_providers: InherentDataProviders,
    job_manager: Arc<RwLock<Option<Arc<dyn JobManager<Job=DefaultJob<B, P::Public>>>>>>,
    params: Params<AccountId, B>,
    foreign_chains: Arc<RwLock<Option<ForeignChain<F>>>>,
) -> Result<impl Future<Item=(), Error=()>, consensus_common::Error> where
    B: Block,
    P: Pair + 'static,
    <P as Pair>::Public: Clone + Debug + Decode + Encode + Send + Sync,
    C: ChainHead<B> + HeaderBackend<B> + ProvideRuntimeApi + 'static,
    <C as ProvideRuntimeApi>::Api: YeePOWApi<B>,
    I: BlockImport<B, Error=consensus_common::Error> + Send + Sync + 'static,
    E: Environment<B> + Send + Sync + 'static,
    <E as Environment<B>>::Error: Debug + Send,
    <<<E as Environment<B>>::Proposer as Proposer<B>>::Create as IntoFuture>::Future: Send + 'static,
    AccountId: Clone + Debug + Decode + Encode + Default + Send + Sync + 'static,
    SO: SyncOracle + Send + Sync + Clone,
    OnExit: Future<Item=(), Error=()>,
    DigestItemFor<B>: CompatibleDigestItem<B, P::Public> + ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<NumberFor<B>, u16>,
    <B as Block>::Hash: From<H256> + Ord,
    F: ServiceFactory + Send + Sync,
    <F as ServiceFactory>::Configuration: ForeignChainConfig + Send+ Sync,
    FactoryFullConfiguration<F>: Clone,
    <<<F as ServiceFactory>::Block as Block>::Header as Header>::Number: From<u64>,
{
    let inner_job_manager = Arc::new(DefaultJobManager::new(
        client.clone(),
        env.clone(),
        inherent_data_providers.clone(),
        local_key.public(),
        block_import.clone(),
        params.shard_extra.clone(),
        params.context.clone(),
        foreign_chains.clone(),
        params.chain_spec_id.clone(),
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
        params.shard_extra.clone(),
    ));
    worker::start_worker(
        worker,
        sync_oracle,
        on_exit,
        params.mine)
}

/// POW chain import queue
pub type PowImportQueue<B> = BasicQueue<B>;


pub trait TriggerExit: Send + Sync{
    fn trigger_restart(&self);
    fn trigger_stop(&self);
}

#[derive(Clone)]
pub struct ShardExtra<AccountId> {
    pub coinbase: Option<AccountId>,
    pub shard_num: u16,
    pub shard_count: u16,
    pub scale_out: Option<ScaleOut>,
    pub trigger_exit: Arc<dyn TriggerExit>,
}

/// Start import queue for POW consensus
pub fn import_queue<F, C, AccountId, AuthorityId>(
    block_import: SharedBlockImport<F::Block>,
    justification_import: Option<SharedJustificationImport<F::Block>>,
    client: Arc<C>,
    inherent_data_providers: InherentDataProviders,
    foreign_chains: Arc<RwLock<Option<ForeignChain<F>>>>,
    shard_extra: ShardExtra<AccountId>,
    context: Context<F::Block>,
    chain_spec_id: String,
) -> Result<PowImportQueue<F::Block>, consensus_common::Error> where
    H256: From<<F::Block as Block>::Hash>,
    F: ServiceFactory + Send + Sync,
    <F as ServiceFactory>::Configuration: ForeignChainConfig + Clone + Send + Sync,
    DigestItemFor<F::Block>: CompatibleDigestItem<F::Block, AuthorityId> + ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<NumberFor<F::Block>, u16>,
    C: ProvideRuntimeApi + 'static + Send + Sync,
    C: HeaderBackend<<F as ServiceFactory>::Block>,
    C: BlockBody<<F as ServiceFactory>::Block>,
    C: BlockchainEvents<<F as ServiceFactory>::Block>,
    C: ChainHead<<F as ServiceFactory>::Block>,
    <C as ProvideRuntimeApi>::Api: ShardingAPI<<F as ServiceFactory>::Block> + YeePOWApi<<F as ServiceFactory>::Block>,
    AccountId: Codec + Send + Sync + Clone + Default + 'static,
    AuthorityId: Decode + Encode + Clone + Send + Sync + 'static,
    substrate_service::config::Configuration<<F as ServiceFactory>::Configuration, <F as ServiceFactory>::Genesis> : Clone,
    <<<F as ServiceFactory>::Block as Block>::Header as Header>::Number: From<u64>,
{
    if let Some(coinbase) = &shard_extra.coinbase {
        register_inherent_data_provider(&inherent_data_providers, coinbase.clone())?;
    }

    let verifier = Arc::new(
        verifier::PowVerifier {
            client,
            inherent_data_providers,
            foreign_chains,
            phantom: PhantomData,
            shard_extra,
            context,
            chain_spec_id,
        }
    );
    Ok(BasicQueue::<F::Block>::new(verifier, block_import, justification_import))
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
