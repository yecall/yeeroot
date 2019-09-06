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
            Decode, Encode,
        },
        traits::{
            DigestItemFor,
            Block,
            ProvideRuntimeApi,
        },
    },
};
use {
    pow_primitives::YeePOWApi,
};

pub use digest::CompatibleDigestItem;
pub use pow::{PowSeal, WorkProof, ProofNonce, ProofMulti};
pub use crate::job::{JobManager, DefaultJobManager, DefaultJob};

mod job;
mod digest;
mod pow;
mod verifier;
mod worker;

pub fn start_pow<B, P, C, I, E, AccountId, SO, OnExit>(
    local_key: Arc<P>,
    client: Arc<C>,
    block_import: Arc<I>,
    env: Arc<E>,
    sync_oracle: SO,
    on_exit: OnExit,
    inherent_data_providers: InherentDataProviders,
    coin_base: AccountId,
    job_manager: Arc<RwLock<Option<Arc<JobManager<Job=DefaultJob<B, P::Public>>>>>>,
    _force_authoring: bool,
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
    AccountId: Clone + Debug + Decode + Encode + Default + Send + 'static,
    SO: SyncOracle + Send + Sync + Clone,
    OnExit: Future<Item=(), Error=()>,
    DigestItemFor<B>: CompatibleDigestItem<B, P::Public>,
{
    let inner_job_manager = Arc::new(DefaultJobManager::new(
        client.clone(),
        env.clone(),
        inherent_data_providers.clone(),
        local_key.public()
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
        local_key.clone(),
        client.clone(),
        block_import,
        sync_oracle.clone(),
        inherent_data_providers.clone(),
        coin_base,
        PhantomData,
    ));
    worker::start_worker(
        worker,
        sync_oracle,
        on_exit)
}

/// POW chain import queue
pub type PowImportQueue<B> = BasicQueue<B>;

/// Start import queue for POW consensus
pub fn import_queue<B, C, AuthorityId>(
    block_import: SharedBlockImport<B>,
    justification_import: Option<SharedJustificationImport<B>>,
    client: Arc<C>,
    inherent_data_providers: InherentDataProviders,
) -> Result<PowImportQueue<B>, consensus_common::Error> where
    B: Block,
    DigestItemFor<B>: CompatibleDigestItem<B, AuthorityId>,
    C: 'static + Send + Sync,
    AuthorityId: Decode + Encode + Send + Sync + 'static,
{
    register_inherent_data_provider(&inherent_data_providers)?;

    let verifier = Arc::new(
        verifier::PowVerifier {
            client,
            inherent_data_providers,
            phantom: PhantomData,
        }
    );
    Ok(BasicQueue::new(verifier, block_import, justification_import))
}

pub fn register_inherent_data_provider(
    inherent_data_providers: &InherentDataProviders,
) -> Result<(), consensus_common::Error> {
    if !inherent_data_providers.has_provider(&srml_timestamp::INHERENT_IDENTIFIER) {
        inherent_data_providers.register_provider(srml_timestamp::InherentDataProvider)
            .map_err(inherent_to_common_error)
    } else {
        Ok(())
    }
}

fn inherent_to_common_error(err: RuntimeString) -> consensus_common::Error {
    consensus_common::ErrorKind::InherentData(err.into()).into()
}
