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

//! Import Queue Verifier for POW chain

use std::{sync::Arc};
use futures::{Future, IntoFuture};

use consensus_common::{
    BlockOrigin, ImportBlock,
    import_queue::Verifier,
};
use runtime_primitives::{
    Justification,
    traits::{Block, Header},
};

use super::AuthorityId;

/// Verifier for POW blocks.
pub struct PowVerifier<C> {
    pub client: Arc<C>,
}

#[forbid(deprecated)]
impl<B: Block, C> Verifier<B> for PowVerifier<C> where
    C: Send + Sync,
{
    fn verify(
        &self,
        origin: BlockOrigin,
        header: <B as Block>::Header,
        justification: Option<Justification>,
        body: Option<Vec<<B as Block>::Extrinsic>>,
    ) -> Result<(ImportBlock<B>, Option<Vec<AuthorityId<B>>>), String> {
        let hash = header.hash();
        let parent_hash = *header.parent_hash();

        // TODO:
        unimplemented!()
    }
}
