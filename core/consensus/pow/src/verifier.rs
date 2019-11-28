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
            NumberFor,
        },
    },
};
use super::CompatibleDigestItem;
use crate::pow::check_proof;
use yee_sharding::{ShardingDigestItem, ScaleOutPhaseDigestItem, ScaleOutPhase};
use log::warn;
use crate::{TriggerExit, ScaleOut, ShardExtra};
use std::thread::sleep;
use std::time::Duration;
use yee_sharding_primitives::utils::shard_num_for;

/// Verifier for POW blocks.
pub struct PowVerifier<C, AccountId, AuthorityId> {
    pub client: Arc<C>,
    pub inherent_data_providers: InherentDataProviders,
    pub phantom: PhantomData<AuthorityId>,
    pub shard_extra: ShardExtra<AccountId>,
}

#[forbid(deprecated)]
impl<B, C, AccountId, AuthorityId> Verifier<B> for PowVerifier<C, AccountId, AuthorityId> where
    B: Block,
    DigestItemFor<B>: CompatibleDigestItem<B, AuthorityId> + ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<NumberFor<B>, u16>,
    C: Send + Sync,
    AccountId: Decode + Encode + Clone + Send + Sync,
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
        let (pre_header, seal) = check_header::<B, AccountId, AuthorityId>(
            header,
            hash,
            self.shard_extra.clone(),
        )?;

        // TODO: verify body

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

/// Check if block header has a valid POW target
fn check_header<B, AccountId, AuthorityId>(
    mut header: B::Header,
    hash: B::Hash,
    shard_extra: ShardExtra<AccountId>
) -> Result<(B::Header, DigestItemFor<B>), String> where
    B: Block,
    DigestItemFor<B>: CompatibleDigestItem<B, AuthorityId> + ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<NumberFor<B>, u16>,
    AuthorityId: Decode + Encode + Clone,
    AccountId: Encode + Decode + Clone,
{
    // pow work proof MUST be last digest item
    let digest_item = match header.digest_mut().pop() {
        Some(x) => x,
        None => return Err(format!("")),
    };
    let seal = digest_item.as_pow_seal().ok_or_else(|| {
        format!("Header {:?} not sealed", hash)
    })?;

    // TODO: check pow_target in seal

    check_shard_info::<B, AccountId>(&header, shard_extra)?;

    check_proof(&header, &seal)?;

    Ok((header, digest_item))
}

pub fn check_shard_info<B, AccountId>(
    header: &B::Header,
    shard_extra: ShardExtra<AccountId>
) -> Result<(), String> where
    B: Block,
    DigestItemFor<B>: ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<NumberFor<B>, u16>,
    AccountId: Encode + Decode + Clone,
{
    let coinbase = shard_extra.coinbase.clone();
    let shard_num = shard_extra.shard_num;
    let shard_count = shard_extra.shard_count;
    let scale_out = shard_extra.scale_out;
    let trigger_exit = shard_extra.trigger_exit;

    //check arg shard info and coinbase when scale out phase committed
    if let Some(ScaleOutPhase::Committed {shard_num: scale_shard_num, shard_count: scale_shard_count}) = header.digest().logs().iter().rev()
        .filter_map(ScaleOutPhaseDigestItem::as_scale_out_phase)
        .next(){

        let target_shard_num = match scale_out{
            Some(scale_out) => scale_out.shard_num,
            None => shard_num,
        };

        if shard_count != scale_shard_count {

            let coinbase_shard_num = shard_num_for(&coinbase, scale_shard_count).expect("qed");
            if target_shard_num != coinbase_shard_num {
                warn!("Stop service for invalid arg coinbase");
                trigger_exit.trigger_stop();
                return Err(format!("Invalid arg coinbase"));
            }

            warn!("Restart service for invalid arg shard info");
            trigger_exit.trigger_restart();

            return Err(format!("Invalid arg shard info"));
        }

    }

    //check header shard info (normal or scaling)
    let (header_shard_num, header_shard_count) : (u16, u16) = header.digest().logs().iter().rev()
        .filter_map(ShardingDigestItem::as_sharding_info)
        .next().expect("non-genesis block always has shard info");

    let original_shard_num = get_original_shard_num(shard_num, shard_count, header_shard_count)?;
    if header_shard_num != original_shard_num {
        return Err(format!("Invalid header shard info"));
    }

    // TODO: check shard info other criteria

    Ok(())
}

fn get_original_shard_num(shard_num: u16, shard_count: u16, original_shard_count: u16) -> Result<u16, String> {

    let mut shard_num = shard_num;
    let mut shard_count = shard_count;
    while shard_count > original_shard_count{
        shard_count = shard_count / 2;
        shard_num = if shard_num >= shard_count { shard_num - shard_count } else { shard_num };
    }

    if shard_count != original_shard_count {
        return Err(format!("Invalid header shard info"));
    }

    Ok(shard_num)
}

#[cfg(test)]
mod tests {

    use crate::verifier::get_original_shard_num;

    #[test]
    fn test_get_original_shard_num() {

        assert_eq!(Ok(0), get_original_shard_num(0u16, 8u16, 8u16));
        assert_eq!(Ok(0), get_original_shard_num(0u16, 8u16, 4u16));
        assert_eq!(Ok(1), get_original_shard_num(1u16, 8u16, 4u16));
        assert_eq!(Ok(2), get_original_shard_num(2u16, 8u16, 4u16));
        assert_eq!(Ok(0), get_original_shard_num(4u16, 8u16, 4u16));
        assert_eq!(Ok(1), get_original_shard_num(13u16, 16u16, 4u16));
        assert_eq!(Err(format!("Invalid header shard info")), get_original_shard_num(5u16, 8u16, 16u16));


    }
}
