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
        fmt::Debug,
        marker::PhantomData,
        sync::{Arc, RwLock},
        time::{Duration, Instant},
    },
    futures::{
        future::{self, Either, Loop},
        Future, IntoFuture,
    },
    log::{warn, info},
    tokio::timer::Delay,
};
use {
    consensus_common::{
        SyncOracle, ImportBlock,
        BlockImport, BlockOrigin, ForkChoiceStrategy,
    },
    inherents::InherentDataProviders,
    runtime_primitives::{
        codec::{Decode, Encode},
        traits::{
            Block, Header,
            Digest, DigestFor, DigestItemFor,
        },
    },
};
use super::{
    CompatibleDigestItem, WorkProof, ProofNonce,
};
use crate::job::{JobManager, DefaultJob};
use crate::pow::check_proof;
use yee_sharding::ShardingDigestItem;

pub trait PowWorker<JM: JobManager> {
    type Error: Debug + Send;
    type OnJob: IntoFuture<Item=JM::Job, Error=Self::Error>;
    type OnWork: IntoFuture<Item=(), Error=Self::Error>;

    fn stop_sign(&self) -> Arc<RwLock<bool>>;

    fn on_start(&self) -> Result<(), Self::Error>;

    fn on_job(&self) -> Self::OnJob;

    fn on_work(&self, iter: u64) -> Self::OnWork;
}

pub struct DefaultWorker<B, I, JM, AuthorityId> {
    job_manager: Arc<JM>,
    block_import: Arc<I>,
    inherent_data_providers: InherentDataProviders,
    stop_sign: Arc<RwLock<bool>>,
    phantom: PhantomData<(B, AuthorityId)>,
}

impl<B, I, JM, AuthorityId> DefaultWorker<B, I, JM, AuthorityId> where
    B: Block,
    JM: JobManager,
{
    pub fn new(
        job_manager: Arc<JM>,
        block_import: Arc<I>,
        inherent_data_providers: InherentDataProviders,
    ) -> Self {
        DefaultWorker {
            job_manager,
            block_import,
            inherent_data_providers,
            stop_sign: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<B, I, JM, AuthorityId> PowWorker<JM> for DefaultWorker<B, I, JM, AuthorityId> where
    B: Block,
    DigestFor<B>: Digest,
    I: BlockImport<B, Error=consensus_common::Error> + Send + Sync + 'static,
    DigestItemFor<B>: CompatibleDigestItem<B, AuthorityId> + ShardingDigestItem<u32>,
    JM: JobManager<Job=DefaultJob<B, AuthorityId>>,
    AuthorityId: Decode + Encode + Clone + 'static,
{
    type Error = consensus_common::Error;
    type OnJob = Box<dyn Future<Item=DefaultJob<B, AuthorityId>, Error=Self::Error> + Send>;
    type OnWork = Box<dyn Future<Item=(), Error=Self::Error> + Send>;

    fn stop_sign(&self) -> Arc<RwLock<bool>> {
        self.stop_sign.clone()
    }

    fn on_start(&self) -> Result<(), consensus_common::Error> {
        super::register_inherent_data_provider(&self.inherent_data_providers)
    }

    fn on_job(&self) -> Self::OnJob {
        self.job_manager.get_job()
    }

    fn on_work(&self,
              iter: u64,
    ) -> Self::OnWork {
        let block_import = self.block_import.clone();

        let job = self.on_job().into_future();

        let on_proposal_block = move |job: DefaultJob<B, AuthorityId>| -> Result<(), consensus_common::Error> {

            let header = job.header;
            let body = job.body;
            let header_num = header.number().clone();
            let header_pre_hash = header.hash();
            let digest_item = job.digest_item;
            let difficulty = digest_item.difficulty;

            info!("block template {} @ {:?} difficulty {:#x}", header_num, header_pre_hash, difficulty);

            // TODO: remove hardcoded
            const PREFIX: &str = "yeeroot-";

            for i in 0_u64..iter {

                let proof = WorkProof::Nonce(ProofNonce::get_with_prefix_len(PREFIX, 12, i));
                let mut seal = digest_item.clone();
                seal.work_proof = proof;

                if let Ok((post_digest, hash)) = check_proof(&header, &seal){
                    let import_block: ImportBlock<B> = ImportBlock {
                        origin: BlockOrigin::Own,
                        header,
                        justification: None,
                        post_digests: vec![post_digest],
                        body: Some(body),
                        finalized: false,
                        auxiliary: Vec::new(),
                        fork_choice: ForkChoiceStrategy::LongestChain,
                    };
                    block_import.import_block(import_block, Default::default())?;

                    info!("block mined @ {} {:?}", header_num, hash);
                    return Ok(());
                }
            }

            Ok(())
        };

        Box::new(
            job
                .map_err(to_common_error)
                .map(move |job| {
                    if let Err(e) = on_proposal_block(job) {
                        warn!("block proposal failed {:?}", e);
                    }
                })
        )
    }
}

pub fn to_common_error<E: Debug>(e: E) -> consensus_common::Error {
    consensus_common::ErrorKind::ClientImport(format!("{:?}", e)).into()
}

pub fn start_worker<W, SO, OnExit, JM>(
    worker: Arc<W>,
    sync_oracle: SO,
    on_exit: OnExit,
) -> Result<impl Future<Item=(), Error=()>, consensus_common::Error> where
    W: PowWorker<JM>,
    SO: SyncOracle,
    OnExit: Future<Item=(), Error=()>,
    JM : JobManager,
{
    worker.on_start().map_err(to_common_error)?;

    let stop_sign = worker.stop_sign();

    info!("worker loop start");
    let work = future::loop_fn((), move |()| {
        let delay = Delay::new(Instant::now() + Duration::from_secs(5));
        let delayed_continue = Either::A(delay.then(|_| future::ok(Loop::Continue(()))));
        let no_delay_stop = Either::B(future::ok(Loop::Break(())));

        match worker.stop_sign().read() {
            Ok(stop_sign) => {
                if *stop_sign {
                    return Either::A(no_delay_stop);
                }
            }
            Err(e) => {
                warn!("work stop sign read error {:?}", e);
                return Either::A(no_delay_stop);
            }
        }

        // worker main loop
        info!("worker one loop start");

        if sync_oracle.is_major_syncing() {
            return Either::A(delayed_continue);
        }

        let task = worker.on_work(10000).into_future();
        Either::B(
            task.then(|_| Delay::new(Instant::now()))
                .then(|_| Ok(Loop::Continue(())))
        )
    });

    Ok(work.select(on_exit).then(move |_| {
        stop_sign.write()
            .map(|mut sign| { *sign = true; })
            .unwrap_or_else(|e| { warn!("write stop sign error : {:?}", e); });

        Ok(())
    }))
}
