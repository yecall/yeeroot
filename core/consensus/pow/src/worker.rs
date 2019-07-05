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

use std::{sync::Arc, time::Duration};
use futures::{future, Future, IntoFuture};
use log::{warn, debug, info};
use client::ChainHead;
use consensus_common::{
    Environment, Proposer, SyncOracle,
};
use inherents::InherentDataProviders;
use runtime_primitives::{
    traits::Block,
};

pub trait PowWorker<B: Block> {
    //type OnJob: IntoFuture<Item=(), Error=consensus_common::Error>;

    fn on_start(&self) -> Result<(), consensus_common::Error>;

    fn on_job(&self, chain_head: B::Header, iter: usize) -> Result<(), consensus_common::Error>;
}

pub struct DefaultWorker<C, E, SO> {
    pub(crate) client: Arc<C>,
    pub(crate) env: Arc<E>,
    pub(crate) sync_oracle: SO,
    pub(crate) inherent_data_providers: InherentDataProviders,
}

impl<B, C, E, SO> PowWorker<B> for DefaultWorker<C, E, SO> where
    B: Block,
    E: Environment<B> + 'static,
    E::Proposer: Proposer<B>,
//    <<E as Environment<B>>::Proposer as Proposer<B>>::Create: IntoFuture,
    SO: SyncOracle,
{
    //type OnJob = Box<Future<Item=(), Error=consensus_common::Error>>;

    fn on_start(&self) -> Result<(), consensus_common::Error> {
        super::register_inherent_data_provider(&self.inherent_data_providers)
    }

    fn on_job(&self,
              chain_head: B::Header,
              iter: usize,
    ) -> Result<(), consensus_common::Error> {
        let client = self.client.clone();
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

        let block = match proposal_work.wait() {
            Ok(b) => b,
            Err(e) => {
                warn!("block build failed {:?}", e);
                return Ok(());
            }
        };
        info!("block template {:?}", block);

        Ok(())
    }
}

pub fn start_worker<B, C, W, SO, OnExit>(
    client: Arc<C>,
    worker: Arc<W>,
    sync_oracle: SO,
    on_exit: OnExit,
) -> Result<impl Future<Item=(), Error=()>, consensus_common::Error> where
    B: Block,
    C: ChainHead<B>,
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
            Ok(x) => {
                info!("succ worker on_job")
            }
            Err(e) => {
                warn!("failed worker on_job {:?}", e);
            }
        }
        std::thread::sleep(std::time::Duration::new(10, 0));


        Ok(future::Loop::Continue(()))
    });

    Ok(work.select(on_exit).then(|_| Ok(())))
}
