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
    ForkChoiceStrategy,
    import_queue::Verifier,
};
use inherents::InherentDataProviders;
use runtime_primitives::{
    Justification,
    traits::{
        Block, Header,
        Digest, DigestItem, DigestItemFor, HashFor,
    },
};

use super::{
    AuthorityId, CompatibleDigestItem, WorkProof,
    pow::check_proof,
};

/// Verifier for POW blocks.
pub struct PowVerifier<C> {
    pub client: Arc<C>,
    pub inherent_data_providers: InherentDataProviders,
}

#[forbid(deprecated)]
impl<B: Block, C> Verifier<B> for PowVerifier<C> where
    DigestItemFor<B>: CompatibleDigestItem,
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

        // check if header has a valid work proof
        let (pre_header, seal) = check_header::<B>(
            header,
            hash,
        )?;

        // TODO: verify body
        // TODO: log

        let import_block = ImportBlock {
            origin,
            header: pre_header,
            justification,
            post_digests: vec![seal],
            body,
            finalized: false,
            auxiliary: Vec::new(),
            fork_choice: ForkChoiceStrategy::LongestChain,
        };
        Ok((import_block, None))
    }
}

/// Check if block header has a valid POW difficulty
fn check_header<B: Block>(
    mut header: B::Header,
    hash: B::Hash,
) -> Result<(B::Header, DigestItemFor<B>), String> where
    DigestItemFor<B>: CompatibleDigestItem,
{
    // pow work proof MUST be last digest item
    let digest_item = match header.digest_mut().pop() {
        Some(x) => x,
        None => return Err(format!("")),
    };
    let work_proof = digest_item.as_pow_seal().ok_or_else(|| {
        // TODO: log
        format!("Header {:?} not sealed", hash)
    })?;

    let pre_hash = header.hash();

    // TODO: remove hardcoded
    let difficulty = primitives::U256::from(0x0000ffff) << 224;

    check_proof::<B>(work_proof, hash, pre_hash, difficulty)?;

    Ok((header, digest_item))
}
