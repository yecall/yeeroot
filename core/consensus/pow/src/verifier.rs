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
        Proof,
        generic,
        traits::{
            Block, Header,
            AuthorityIdFor, Digest, DigestItemFor,
            NumberFor,
            BlakeTwo256,
            ProvideRuntimeApi,
            Zero,
            One,
            As,
        },
    },
    substrate_service::{
        ServiceFactory,
        FactoryBlock,
    },
    client::{
        self,
        BlockchainEvents,
        ChainHead,
        blockchain::HeaderBackend,
        BlockBody,
    },
    yee_sharding_primitives::ShardingAPI,
    util::relay_decode::RelayTransfer,
    foreign_chain::{ForeignChain, ForeignChainConfig},
    yee_runtime::{AccountId, BalancesCall},
    pow_primitives::YeePOWApi,
};
use super::CompatibleDigestItem;
use super::worker::to_common_error;
use crate::pow::{PowSeal, check_proof, gen_extrinsic_proof};
use yee_sharding::{ShardingDigestItem, ScaleOutPhaseDigestItem, ScaleOutPhase};
use crate::{TriggerExit, ScaleOut, ShardExtra};
use std::thread::sleep;
use std::time::Duration;
use yee_sharding_primitives::utils::shard_num_for;
use merkle_light::proof::Proof as MLProof;
use merkle_light::merkle::MerkleTree;
use yee_merkle::{ProofHash, ProofAlgorithm, MultiLayerProof};
use ansi_term::Colour;
use log::{debug, info, warn};
use parking_lot::RwLock;
use primitives::H256;
// use yee_runtime::BlockId;

/// Verifier for POW blocks.
pub struct PowVerifier<F: ServiceFactory, C, AccountId, AuthorityId> {
    pub client: Arc<C>,
    pub inherent_data_providers: InherentDataProviders,
    pub foreign_chains: Arc<RwLock<Option<ForeignChain<F>>>>,
    pub phantom: PhantomData<AuthorityId>,
    pub shard_extra: ShardExtra<AccountId>,
}

#[forbid(deprecated)]
impl<F, C, AccountId, AuthorityId> Verifier<F::Block> for PowVerifier<F, C, AccountId, AuthorityId> where
    DigestItemFor<F::Block>: CompatibleDigestItem<F::Block, AuthorityId> + ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<NumberFor<F::Block>, u16>,
    C: Send + Sync,
    AccountId: Decode + Encode + Clone + Send + Sync + Default,
    AuthorityId: Decode + Encode + Clone + Send + Sync,
    F: ServiceFactory + Send + Sync,
    <F as ServiceFactory>::Configuration: ForeignChainConfig + Clone + Send + Sync,
    C: ProvideRuntimeApi,
    C: HeaderBackend<<F as ServiceFactory>::Block>,
    C: BlockBody<<F as ServiceFactory>::Block>,
    C: BlockchainEvents<<F as ServiceFactory>::Block>,
    C: ChainHead<<F as ServiceFactory>::Block>,
    <C as ProvideRuntimeApi>::Api: ShardingAPI<<F as ServiceFactory>::Block> + YeePOWApi<<F as ServiceFactory>::Block>,
    H256: From<<F::Block as Block>::Hash>,
    substrate_service::config::Configuration<<F as ServiceFactory>::Configuration, <F as ServiceFactory>::Genesis>: Clone,
{
    fn verify(
        &self,
        origin: BlockOrigin,
        header: <F::Block as Block>::Header,
        justification: Option<Justification>,
        proof: Option<Proof>,
        body: Option<Vec<<F::Block as Block>::Extrinsic>>,
    ) -> Result<(ImportBlock<F::Block>, Option<Vec<AuthorityIdFor<F::Block>>>), String> {
        let number = header.number().clone();
        let hash = header.hash();
        let _parent_hash = *header.parent_hash();

        // check if header has a valid work proof
        let (pre_header, seal) = self.check_header(
            header,
            hash.clone(),
            self.shard_extra.clone(),
        )?;

        let proof_seal = seal.as_pow_seal().ok_or_else(|| {
            format!("Header {:?} not sealed", hash)
        })?;
        let p_h = proof_seal.relay_proof.clone();

        // check proof.
        match self.check_proof(proof.clone(), p_h) {
            Err(e) => {
                warn!("{}, number:{}, hash:{}", Colour::Red.paint("Proof validate failed"), number, hash.clone());
                return Err(e);
            }
            Ok(()) => { debug!("{}, number:{}, hash:{}", Colour::Green.paint("Proof validated"), number, hash.clone()); }
        }
        let mut res_proof = proof;
        match body.as_ref() {
            Some(exs) => {
                /// TODO validate body

                // check proof root
                let (root, proof) = gen_extrinsic_proof::<F::Block>(&pre_header, &exs);
                if root != p_h {
                    return Err("Proof is invalid.".to_string());
                }
                res_proof = Some(proof);
                let logs = pre_header.digest().logs();
                // check relay extrinsic.
                let check_relay = self.check_relay_transfer(logs, exs);
                if check_relay.is_err() {
                    return Err(check_relay.err().unwrap());
                }
            }
            None => {}
        }
        if justification.is_some() {
            debug!("justification is some");
        }
        let import_block = ImportBlock {
            origin,
            header: pre_header,
            justification,
            proof: res_proof,
            post_digests: vec![seal],
            body,
            finalized: false,
            auxiliary: Vec::new(),
            fork_choice: ForkChoiceStrategy::LongestChain,
        };
        Ok((import_block, None))
    }
}

impl<F, C, AccountId, AuthorityId> PowVerifier<F, C, AccountId, AuthorityId> where
    DigestItemFor<F::Block>: CompatibleDigestItem<F::Block, AuthorityId> + ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<NumberFor<F::Block>, u16>,
    C: Send + Sync,
    AccountId: Decode + Encode + Clone + Send + Sync + Default,
    AuthorityId: Decode + Encode + Clone + Send + Sync,
    F: ServiceFactory + Send + Sync,
    <F as ServiceFactory>::Configuration: ForeignChainConfig + Clone + Send + Sync,
    C: ProvideRuntimeApi + HeaderBackend<<F as ServiceFactory>::Block>,
    C: BlockBody<<F as ServiceFactory>::Block>,
    C: BlockchainEvents<<F as ServiceFactory>::Block>,
    C: ChainHead<<F as ServiceFactory>::Block>,
    <C as ProvideRuntimeApi>::Api: ShardingAPI<<F as ServiceFactory>::Block> + YeePOWApi<<F as ServiceFactory>::Block>,
    H256: From<<F::Block as Block>::Hash>,
    substrate_service::config::Configuration<<F as ServiceFactory>::Configuration, <F as ServiceFactory>::Genesis>: Clone,
{
    fn check_proof(&self, proof: Option<Proof>, p_h: H256) -> Result<(), String> {
        if let Some(proof) = proof.clone() {
            let mut validate_proof = false;
            if let Ok(mlp) = MultiLayerProof::from_bytes(proof.as_slice()) {
                // check proof root.
                let checked = match mlp.layer2_root() {
                    Some(root) => root == p_h,
                    None => false
                };
                if !checked {
                    return Err("Proof is invalid.".to_string());
                }
                // check proof self.
                if let Ok(mt_proof) = MLProof::from_bytes(mlp.layer2_proof.as_ref().unwrap().as_slice()) {
                    let mt_proof: MLProof<ProofHash<BlakeTwo256>> = mt_proof;
                    if mt_proof.validate::<ProofAlgorithm<BlakeTwo256>>() {
                        validate_proof = true;
                    }
                }
            }
            if !validate_proof {
                return Err("Proof validate failed.".to_string());
            }
        }
        Ok(())
    }

    fn check_relay_transfer(&self, logs: &[DigestItemFor<F::Block>], exs: &[<F::Block as Block>::Extrinsic]) -> Result<(), String> {
        let err_str = "Block contains invalid extrinsic.";
        let shard_info: Option<(u16, u16)> = logs.iter().rev()
            .filter_map(ShardingDigestItem::as_sharding_info)
            .next();
        let (tc, cs) = match shard_info {
            Some(info) => info,
            None => { return Err("Can't get shard info in header".to_string()); }
        };
        for tx in exs {
            let rt = RelayTransfer::decode(tx.encode().as_slice());
            if rt.is_some() {
                let rt: RelayTransfer<AccountId, u128, <F::Block as Block>::Hash> = rt.unwrap();
                let h = rt.hash();
                let id = generic::BlockId::hash(h);
                let src = rt.transfer.sender();
                if let Some(ds) = yee_sharding_primitives::utils::shard_num_for(&src, tc as u16) {
                    if let Some(lc) = self.foreign_chains.read().as_ref().unwrap().get_shard_component(ds) {
                        let proof = lc.client().proof(&id).map_err(|_| err_str)?;
                        match proof {
                            Some(proof) => {
                                let proof = MultiLayerProof::from_bytes(proof.as_slice()).map_err(|_| err_str)?;
                                if proof.contains(ds, h) {
                                    continue;
                                }
                            }
                            None => { return Err(err_str.to_string()); }
                        }
                    }
                }
                panic!("Internal error. Get shard num or component failed.");
            }
        }
        Ok(())
    }

    /// Check if block header has a valid POW target
    fn check_header(&self, mut header: <F::Block as Block>::Header, hash: <F::Block as Block>::Hash, shard_extra: ShardExtra<AccountId>) -> Result<(<F::Block as Block>::Header, DigestItemFor<F::Block>), String> {
        // pow work proof MUST be last digest item
        let digest_item = match header.digest_mut().pop() {
            Some(x) => x,
            None => return Err(format!("")),
        };
        let seal = digest_item.as_pow_seal().ok_or_else(|| {
            format!("Header {:?} not sealed", hash)
        })?;

        match self.check_pow_target(&header, &seal) {
            Err(err) => {
                warn!("validate pow target failed. {}", err);
                return Err(err);
            }
            _ => {}
        }

        check_shard_info::<F::Block, AccountId>(&header, shard_extra)?;

        check_proof(&header, &seal)?;

        Ok((header, digest_item))
    }

    /// check pow_target in seal
    fn check_pow_target(&self, header: &<F::Block as Block>::Header, seal: &PowSeal<F::Block, AuthorityId>) -> Result<(), String> {
        let num = *header.number();
        let parent_id = generic::BlockId::<F::Block>::hash(*header.parent_hash());
        let api = self.client.runtime_api();
        let one = One::one();
        let genesis_pow_target = api.genesis_pow_target(&parent_id)
            .map_err(|e| format!("{:?}", e))?;
        let adj = api.pow_target_adj(&parent_id)
            .map_err(|e| format!("{:?}", e))?;

        let pow_target = if num == one {
            genesis_pow_target
        } else {
            // in same period
            if (num - one) % adj != Zero::zero() {
                header.digest().logs().iter().rev()
                    .filter_map(CompatibleDigestItem::as_pow_seal).next()
                    .and_then(|seal| Some(seal.pow_target))
                    .unwrap_or(genesis_pow_target)
            } else { // change pow difficulty
                let parent = self.client.header(parent_id)
                    .expect("parent block must exist for sealer; qed")
                    .expect("parent block must exist for sealer; qed");
                let parent_seal = parent.digest().logs().iter().rev()
                    .filter_map(CompatibleDigestItem::as_pow_seal).next()
                    .expect("Seal must exist when adjustment comes; qed");
                let ancestor_id = generic::BlockId::<F::Block>::number(num - adj);
                let ancestor_header = self.client.header(ancestor_id)
                    .expect("parent block must exist for sealer; qed")
                    .expect("parent block must exist for sealer; qed");
                let ancestor_seal = ancestor_header.digest().logs().iter().rev()
                    .filter_map(CompatibleDigestItem::as_pow_seal).next()
                    .expect("Seal must exist when adjustment comes; qed");

                let parent_time = api.target_block_time(&parent_id)
                    .map_err(|e| format!("{:?}", e))?;
                let time_gap = parent_seal.timestamp - ancestor_seal.timestamp;
                let block_gap = adj.as_();
                let expected_gap = parent_time * 1000 * block_gap;
                (parent_seal.pow_target / expected_gap) * time_gap
            }
        };
        if pow_target == seal.pow_target {
            return Ok(());
        } else {
            return Err("pow target validate failed".to_string());
        }
    }
}

pub fn check_shard_info<B, AccountId>(
    header: &B::Header,
    shard_extra: ShardExtra<AccountId>,
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
    if let Some(ScaleOutPhase::Committed { shard_num: scale_shard_num, shard_count: scale_shard_count }) = header.digest().logs().iter().rev()
        .filter_map(ScaleOutPhaseDigestItem::as_scale_out_phase)
        .next() {
        let target_shard_num = match scale_out {
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
    let (header_shard_num, header_shard_count): (u16, u16) = header.digest().logs().iter().rev()
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
    while shard_count > original_shard_count {
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
