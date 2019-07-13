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

use std::{marker::PhantomData, sync::Arc};
use futures::{Future, IntoFuture};
use client::ChainHead;
use consensus_common::{
    BlockImport, Environment, Proposer, SyncOracle,
    import_queue::{
        BasicQueue,
        SharedBlockImport, SharedJustificationImport,
    },
};
use inherents::{InherentDataProviders, RuntimeString};
use runtime_primitives::{
    codec::{
        Decode, Encode,
    },
    traits::{
        AuthorityIdFor, DigestItemFor,
        Block, Header,
    },
};

pub use digest::CompatibleDigestItem;
pub use pow::{PowSeal, WorkProof, ProofNonce};

mod digest;
mod pow;
mod verifier;
mod worker;

type AuthorityId<B> = AuthorityIdFor<B>;

pub fn start_pow<B, C, I, E, AccountId, SO, OnExit>(
    client: Arc<C>,
    block_import: Arc<I>,
    env: Arc<E>,
    sync_oracle: SO,
    on_exit: OnExit,
    inherent_data_providers: InherentDataProviders,
    coin_base: AccountId,
    force_authoring: bool,
) -> Result<impl Future<Item=(), Error=()>, consensus_common::Error> where
    B: Block,
    C: ChainHead<B>,
    I: BlockImport<B, Error=consensus_common::Error>,
    E: Environment<B> + 'static,
    AccountId: Clone + Decode + Encode + Default,
    SO: SyncOracle + Send + Sync + Clone,
    OnExit: Future<Item=(), Error=()>,
    DigestItemFor<B>: CompatibleDigestItem<AccountId>,
{
    let worker = worker::DefaultWorker {
        client: client.clone(),
        block_import,
        env,
        sync_oracle: sync_oracle.clone(),
        inherent_data_providers: inherent_data_providers.clone(),
        coin_base,
        phantom: PhantomData,
    };
    worker::start_worker::<_, _, I, _, _, _>(
        client,
        Arc::new(worker),
        sync_oracle,
        on_exit)
}

/// POW chain import queue
pub type PowImportQueue<B> = BasicQueue<B>;

/// Start import queue for POW consensus
pub fn import_queue<B, C, AccountId>(
    block_import: SharedBlockImport<B>,
    justification_import: Option<SharedJustificationImport<B>>,
    client: Arc<C>,
    inherent_data_providers: InherentDataProviders,
) -> Result<PowImportQueue<B>, consensus_common::Error> where
    B: Block,
    DigestItemFor<B>: CompatibleDigestItem<AccountId>,
    C: 'static + Send + Sync,
    AccountId: Decode + Encode + Send + Sync + 'static,
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

fn register_inherent_data_provider(
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
