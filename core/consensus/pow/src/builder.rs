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
    futures::{
        Future, IntoFuture,
        future,
    },
    log::warn,
    std::{
        fmt::Debug,
        marker::PhantomData,
        sync::Arc,
    },
};
use {
    client::ChainHead,
    consensus_common::{
        Environment, Proposer,
    },
    inherents::InherentDataProviders,
    runtime_primitives::{
        traits::Block,
    },
};
use {
    super::{
        worker::to_common_error,
    },
};
use std::time::Duration;

pub struct JobTemplateBuilder<B, C, E> where
{
    client: Arc<C>,
    env: Arc<E>,
    inherent_data_providers: InherentDataProviders,
    phantom: PhantomData<B>,
}

impl<B, C, E> JobTemplateBuilder<B, C, E> where
    B: Block,
    C: ChainHead<B>,
    E: Environment<B> + 'static,
    <E as Environment<B>>::Proposer: Proposer<B>,
    <E as Environment<B>>::Error: Debug,
    <<E as Environment<B>>::Proposer as Proposer<B>>::Create: IntoFuture<Item=B>,
    <<<E as Environment<B>>::Proposer as Proposer<B>>::Create as IntoFuture>::Future: Send + 'static,
{
    pub fn new(
        client: Arc<C>, env: Arc<E>, inherent_data_providers: InherentDataProviders,
        phantom: PhantomData<B>,
    ) -> Self {
        Self {
            client,
            env,
            inherent_data_providers,
            phantom,
        }
    }

    pub fn get_template(&self) -> Box<dyn Future<Item=B, Error=consensus_common::Error> + Send> {
        let get_data = || {
            let chain_head = self.client.best_block_header()
                .map_err(to_common_error)?;
            let proposer = self.env.init(&chain_head, &vec![])
                .map_err(to_common_error)?;
            let inherent_data = self.inherent_data_providers.create_inherent_data()
                .map_err(to_common_error)?;
            Ok((proposer, inherent_data))
        };
        let (proposer, inherent_data) = match get_data() {
            Ok((p, d)) => (p, d),
            Err(e) => {
                warn!("failed to get proposer {:?}", e);
                return Box::new(future::err(e));
            }
        };

        Box::new(
            proposer.propose(inherent_data, Duration::from_secs(10))
                .into_future()
                .map_err(to_common_error)
        )
    }
}
