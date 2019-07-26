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
            Digest, DigestItemFor,
        },
    },
};

use super::{
    AuthorityId, CompatibleDigestItem,
    pow::check_seal,
};

/// Verifier for POW blocks.
pub struct PowVerifier<C, AccountId> {
    pub client: Arc<C>,
    pub inherent_data_providers: InherentDataProviders,
    pub phantom: PhantomData<AccountId>,
}

#[forbid(deprecated)]
impl<B, C, AccountId> Verifier<B> for PowVerifier<C, AccountId> where
    B: Block,
    DigestItemFor<B>: CompatibleDigestItem<AccountId>,
    C: Send + Sync,
    AccountId: Decode + Encode + Send + Sync,
{
    fn verify(
        &self,
        origin: BlockOrigin,
        header: <B as Block>::Header,
        justification: Option<Justification>,
        body: Option<Vec<<B as Block>::Extrinsic>>,
    ) -> Result<(ImportBlock<B>, Option<Vec<AuthorityId<B>>>), String> {
        let hash = header.hash();
        let _parent_hash = *header.parent_hash();

        // check if header has a valid work proof
        let (pre_header, seal) = check_header::<B, AccountId>(
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
fn check_header<B, AccountId>(
    mut header: B::Header,
    hash: B::Hash,
) -> Result<(B::Header, DigestItemFor<B>), String> where
    B: Block,
    DigestItemFor<B>: CompatibleDigestItem<AccountId>,
    AccountId: Decode + Encode,
{
    // pow work proof MUST be last digest item
    let digest_item = match header.digest_mut().pop() {
        Some(x) => x,
        None => return Err(format!("")),
    };
    let seal = digest_item.as_pow_seal().ok_or_else(|| {
        // TODO: log
        format!("Header {:?} not sealed", hash)
    })?;

    let pre_hash = header.hash();

    // TODO: check seal.difficulty

    check_seal::<B, AccountId>(seal, hash, pre_hash)?;

    Ok((header, digest_item))
}
