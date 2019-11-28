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
};
use super::CompatibleDigestItem;
use crate::pow::{check_proof, gen_extrinsic_proof};
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
use yee_runtime::BlockId;

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
    C : ProvideRuntimeApi,
    C : HeaderBackend<<F as ServiceFactory>::Block>,
    C : BlockBody<<F as ServiceFactory>::Block>,
    C : BlockchainEvents<<F as ServiceFactory>::Block>,
    C : ChainHead<<F as ServiceFactory>::Block>,
    <C as ProvideRuntimeApi>::Api: ShardingAPI<<F as ServiceFactory>::Block>,
    H256: From<<F::Block as Block>::Hash>,
    substrate_service::config::Configuration<<F as ServiceFactory>::Configuration, <F as ServiceFactory>::Genesis> : Clone,
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
        let (pre_header, seal, p_h) = check_header::<F::Block, AccountId, AuthorityId>(
            header,
            hash.clone(),
            self.shard_extra.clone(),
        )?;

        // TODO: verify body

        // check proof.
        let mut validate_proof = false;
        if let Some(proof) = proof.clone() {
            if let Ok(mlp) = MultiLayerProof::from_bytes(proof.as_slice()){
                // check proof root.
                let root = mlp.layer2_root();
                if root.is_none() || root.unwrap() != p_h {
                    return Err("Proof is invalid.".to_string());
                }
                // check proof self.
                if let Ok(mt_proof) = MLProof::from_bytes(mlp.layer2_proof.as_slice()) {
                    let mt_proof: MLProof<ProofHash<BlakeTwo256>> = mt_proof;
                    if mt_proof.validate::<ProofAlgorithm<BlakeTwo256>>() {
                        validate_proof = true;
                    }
                }
            }
        }
        if !validate_proof {
            warn!("{}, number:{}, hash:{}", Colour::Red.paint("Proof validate failed"), number, hash.clone());
            return Err("Proof validate failed.".to_string());
        } else {
            debug!("{}, number:{}, hash:{}", Colour::Green.paint("Proof validated"), number, hash.clone());
        }
        let mut res_proof = proof;
        let foreign_chains = self.foreign_chains.clone();
        // let client = self.client.clone();
        let digest = pre_header.digest().clone();
        if body.is_some() {
            let check_relay_extrinsic = move |exs: Vec<<F::Block as Block>::Extrinsic>| -> Result<(), String> {
                let err = Err("Block contains invalid extrinsic.".to_string());
                let shard_info : Option<(u16, u16)> = digest.logs().iter().rev()
                    .filter_map(ShardingDigestItem::as_sharding_info)
                    .next();
                if shard_info.is_none() {
                    return Err("Can't get shard info in header".to_string());
                }
                let shard_info = shard_info.unwrap();
                let (tc, cs) = (shard_info.0, shard_info.1);
                for tx in exs {
                    let rt = RelayTransfer::decode(tx.encode());
                    if rt.is_some() {
                        let rt: RelayTransfer<AccountId, u128, <F::Block as Block>::Hash> = rt.unwrap();
                        let h = rt.hash();
                        let id = generic::BlockId::hash(h);
                        let src = rt.transfer.sender();
                        if let Some(ds) = yee_sharding_primitives::utils::shard_num_for(&src, tc as u16) {
                            if let Some(lc) = foreign_chains.read().as_ref().unwrap().get_shard_component(ds) {
                                let proof = lc.client().proof(&id);
                                if proof.is_err() {
                                    return err;
                                }
                                let proof = proof.unwrap();
                                if proof.is_none() {
                                    return err;
                                }
                                let proof = proof.unwrap();
                                let proof =  MultiLayerProof::from_bytes(proof.as_slice());
                                if proof.is_err() {
                                    return err;
                                }
                                let proof = proof.unwrap();
                                if proof.contains(ds, h){
                                    continue;
                                }
                                return err;
                            }
                        }
                        panic!("Internal error. Get shard num or component failed.");
                    }
                }

                Ok(())
            };

            let exs = body.clone().unwrap();
            // check proof root
            let (root, proof) = gen_extrinsic_proof::<F::Block>(&pre_header, &exs);
            if root != p_h {
                return Err("Proof is invalid.".to_string());
            }
            res_proof = Some(proof);
            // check relay extrinsic.
            let check_relay : Result<(), String> = check_relay_extrinsic(exs);
            if check_relay.is_err() {
                return Err(check_relay.err().unwrap());
            }
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

/// Check if block header has a valid POW target
fn check_header<B, AccountId, AuthorityId>(
    mut header: B::Header,
    hash: B::Hash,
    shard_extra: ShardExtra<AccountId>
) -> Result<(B::Header, DigestItemFor<B>, H256), String> where
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

    Ok((header, digest_item, seal.relay_proof.clone()))
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

    //check header shard info
    let shard_info : (u16, u16) = header.digest().logs().iter().rev()
        .filter_map(ShardingDigestItem::as_sharding_info)
        .next().expect("non-genesis block always has shard info");

    if (shard_num, shard_count) != shard_info {
        return Err(format!("Invalid header shard info"));
    }

    // TODO: check shard info other criteria

    Ok(())
}
