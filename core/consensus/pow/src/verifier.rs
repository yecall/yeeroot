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

use {
    std::{marker::PhantomData, sync::Arc},
};
use {
    consensus_common::{
        BlockOrigin, ImportBlock,
        ForkChoiceStrategy,
        import_queue::Verifier,
    },
    inherents::InherentDataProviders,
    runtime_primitives::{
        codec::{Decode, Encode},
        Justification,
        traits::{
            Block, Header,
            AuthorityIdFor, Digest, DigestItemFor,
        },
    },
};
use super::CompatibleDigestItem;
use crate::pow::check_proof;
use yee_sharding::ShardingDigestItem;

/// Verifier for POW blocks.
pub struct PowVerifier<C, AuthorityId> {
    pub client: Arc<C>,
    pub inherent_data_providers: InherentDataProviders,
    pub phantom: PhantomData<AuthorityId>,
}

#[forbid(deprecated)]
impl<B, C, AuthorityId> Verifier<B> for PowVerifier<C, AuthorityId> where
    B: Block,
    DigestItemFor<B>: CompatibleDigestItem<B, AuthorityId> + ShardingDigestItem<u16>,
    C: Send + Sync,
    AuthorityId: Decode + Encode + Clone + Send + Sync,
{
    fn verify(
        &self,
        origin: BlockOrigin,
        header: <B as Block>::Header,
        justification: Option<Justification>,
        body: Option<Vec<<B as Block>::Extrinsic>>,
    ) -> Result<(ImportBlock<B>, Option<Vec<AuthorityIdFor<B>>>), String> {
        let hash = header.hash();
        let _parent_hash = *header.parent_hash();

        // check if header has a valid work proof
        let (pre_header, seal) = check_header::<B, AuthorityId>(
            header,
            hash,
        )?;

        // TODO: verify body
        let proof = None;

        let import_block = ImportBlock {
            origin,
            header: pre_header,
            justification,
            proof,
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
fn check_header<B, AccountId>(
    mut header: B::Header,
    hash: B::Hash,
) -> Result<(B::Header, DigestItemFor<B>), String> where
    B: Block,
    DigestItemFor<B>: CompatibleDigestItem<B, AccountId> + ShardingDigestItem<u16>,
    AccountId: Decode + Encode + Clone,
{
    // pow work proof MUST be last digest item
    let digest_item = match header.digest_mut().pop() {
        Some(x) => x,
        None => return Err(format!("")),
    };
    let seal = digest_item.as_pow_seal().ok_or_else(|| {
        format!("Header {:?} not sealed", hash)
    })?;

    // TODO: check seal.difficulty

    check_proof(&header, &seal)?;

    Ok((header, digest_item))
}
