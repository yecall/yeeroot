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
    std::{marker::PhantomData, sync::Arc, time::Duration},
    futures::{future, Future, IntoFuture},
    log::{warn, debug, info},
};
use {
    client::ChainHead,
    consensus_common::{
        Environment, Proposer, SyncOracle, ImportBlock,
        BlockImport, BlockOrigin, ForkChoiceStrategy,
    },
    inherents::InherentDataProviders,
    runtime_primitives::{
        codec::{Decode, Encode},
        traits::{
            Block, Header,
            Digest, DigestItemFor,
        },
    },
};
use super::{
    CompatibleDigestItem, WorkProof, ProofNonce,
    pow::{check_seal, PowSeal},
};

pub trait PowWorker<B: Block> {
    //type OnJob: IntoFuture<Item=(), Error=consensus_common::Error>;

    fn on_start(&self) -> Result<(), consensus_common::Error>;

    fn on_job(&self, chain_head: B::Header, iter: u64) -> Result<(), consensus_common::Error>;
}

pub struct DefaultWorker<C, I, E, AccountId, SO> {
    pub(crate) client: Arc<C>,
    pub(crate) block_import: Arc<I>,
    pub(crate) env: Arc<E>,
    pub(crate) sync_oracle: SO,
    pub(crate) inherent_data_providers: InherentDataProviders,
    pub(crate) coin_base: AccountId,
    pub(crate) phantom: PhantomData<AccountId>,
}

impl<B, C, I, E, AccountId, SO> PowWorker<B> for DefaultWorker<C, I, E, AccountId, SO> where
    B: Block,
    <B::Header as Header>::Digest: Digest,
    I: BlockImport<B, Error=consensus_common::Error>,
    E: Environment<B> + 'static,
    <E as Environment<B>>::Proposer: Proposer<B>,
    <<E as Environment<B>>::Proposer as Proposer<B>>::Create: IntoFuture<Item=B>,
    AccountId: Clone + Decode + Encode + Default,
    SO: SyncOracle,
    DigestItemFor<B>: CompatibleDigestItem<AccountId>,
{
    //type OnJob = Box<Future<Item=(), Error=consensus_common::Error>>;

    fn on_start(&self) -> Result<(), consensus_common::Error> {
        super::register_inherent_data_provider(&self.inherent_data_providers)
    }

    fn on_job(&self,
              chain_head: B::Header,
              iter: u64,
    ) -> Result<(), consensus_common::Error> {
        let client = self.client.clone();
        let block_import = self.block_import.clone();
        let env = self.env.clone();

        let proposer = match env.init(&chain_head, &Vec::new()) {
            Ok(p) => p,
            Err(e) => {
//                warn!("failed to create block {:?}", e);
                return Ok(());
            }
        };
        let inherent_data = self.inherent_data_providers.create_inherent_data()
            .map_err(super::inherent_to_common_error)?;
        let remaining_duration = Duration::new(10, 0);

        let proposal_work = proposer.propose(
            inherent_data, remaining_duration,
        ).into_future();

        let block: B = match proposal_work.wait() {
            Ok(b) => b,
            Err(e) => {
                warn!("block build failed {:?}", e);
                return Ok(());
            }
        };
        let (header, body) = block.deconstruct();
        let header_num = header.number().clone();
        let header_pre_hash = header.hash();
        info!("block template {} @ {:?}", header_num, header_pre_hash);

        // TODO: remove hardcoded
        const PREFIX: &str = "yeeroot-";

        for i in 0_u64..iter {
            let mut work_header = header.clone();
            let proof = WorkProof::Nonce(ProofNonce::get_with_prefix_len(PREFIX, 12, i));
            let seal = PowSeal {
                difficulty: primitives::U256::from(0x0000ffff) << 224,
                coin_base: self.coin_base.clone(),
                work_proof: proof,
            };
            let item = <DigestItemFor<B> as CompatibleDigestItem<AccountId>>::pow_seal(seal.clone());
            work_header.digest_mut().push(item);

            let post_hash = work_header.hash();
            if let Ok(_) = check_seal::<B, AccountId>(seal, post_hash, header_pre_hash) {
                let valid_seal = work_header.digest_mut().pop().expect("must exists");
                let import_block: ImportBlock<B> = ImportBlock {
                    origin: BlockOrigin::Own,
                    header,
                    justification: None,
                    post_digests: vec![valid_seal],
                    body: Some(body),
                    finalized: false,
                    auxiliary: Vec::new(),
                    fork_choice: ForkChoiceStrategy::LongestChain,
                };
                block_import.import_block(import_block, Default::default())?;

                info!("block mined @ {} {:?}", header_num, post_hash);
                return Ok(());
            }
        }

        Ok(())
    }
}

pub fn start_worker<B, C, I, W, SO, OnExit>(
    client: Arc<C>,
    worker: Arc<W>,
    sync_oracle: SO,
    on_exit: OnExit,
) -> Result<impl Future<Item=(), Error=()>, consensus_common::Error> where
    B: Block,
    C: ChainHead<B>,
    I: BlockImport<B>,
    W: PowWorker<B>,
//W::OnJob: IntoFuture<Item=(), Error=consensus_common::Error>,
    SO: SyncOracle,
    OnExit: Future<Item=(), Error=()>,
{
    worker.on_start()?;

    info!("worker loop start");
    let work = future::loop_fn((), move |()| {
        // worker main loop
        info!("worker one loop start");

        if sync_oracle.is_major_syncing() {
            std::thread::sleep(std::time::Duration::new(5, 0));
            return Ok(future::Loop::Continue(()));
        }

        let chain_head = match client.best_block_header() {
            Ok(x) => x,
            Err(e) => {
                warn!("failed to get chain head {:?}", e);
                std::thread::sleep(std::time::Duration::new(5, 0));
                return Ok(future::Loop::Continue(()));
            }
        };

        let task = worker.on_job(chain_head, 10000).into_future();
        match task.wait() {
            Ok(_) => {
                info!("succ worker on_job")
            }
            Err(e) => {
                warn!("failed worker on_job {:?}", e);
            }
        }

        Ok(future::Loop::Continue(()))
    });

    Ok(work.select(on_exit).then(|_| Ok(())))
}
