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

/// Import Queue Verifier for POW chain

use std::{sync::Arc};
use futures::{Future, IntoFuture};

use consensus_common::{
    BlockOrigin, ImportBlock,
    import_queue::Verifier,
};
use runtime_primitives::Justification;

use super::{
    AuthorityId,
    Block, Header,
};

/// Verifier for POW blocks.
pub struct PowVerifier<C, E> {
    pub client: Arc<C>,
    pub extra: E,
}

#[forbid(deprecated)]
impl<B: Block, C, E> Verifier<B> for PowVerifier<C, E> where
    C: Send + Sync,
    E: ExtraVerification<B>,
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

        // extra verifier runs besides normal verification
        let extra_verification = self.extra.verify(
            &header,
            body.as_ref().map(|x| &x[..]),
        );

        // wait and check extra verification result
        extra_verification.into_future().wait()?;

        unimplemented!()
    }
}

/// Extra verification for POW blocks in native code.
pub trait ExtraVerification<B: Block>: Send + Sync {
    /// Future for verification
    type Verified: IntoFuture<Item=(), Error=String>;

    /// extra verification implementation
    fn verify(
        &self,
        header: &B::Header,
        body: Option<&[B::Extrinsic]>,
    ) -> Self::Verified;
}

/// No-op extra verification.
pub struct NothingExtra;

impl<B: Block> ExtraVerification<B> for NothingExtra {
    type Verified = Result<(), String>;

    fn verify(&self, _: &<B as Block>::Header, _: Option<&[<B as Block>::Extrinsic]>) -> Self::Verified {
        Ok(())
    }
}
