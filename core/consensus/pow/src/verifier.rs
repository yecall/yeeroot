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
use crate::digest::ProofDigestItem;
use yee_sharding::ShardingDigestItem;
use merkle_light::proof::Proof as MLProof;
use merkle_light::merkle::MerkleTree;
use yee_merkle::{ProofHash, ProofAlgorithm, MultiLayerProof};
use ansi_term::Colour;
use log::{debug, warn};
use parking_lot::RwLock;
use primitives::H256;
use yee_runtime::BlockId;

/// Verifier for POW blocks.
pub struct PowVerifier<F: ServiceFactory, C, AuthorityId> {
    pub client: Arc<C>,
    pub inherent_data_providers: InherentDataProviders,
    pub foreign_chains: Arc<RwLock<Option<ForeignChain<F, C>>>>,
    pub phantom: PhantomData<AuthorityId>,
}

#[forbid(deprecated)]
impl<F, C, AuthorityId> Verifier<F::Block> for PowVerifier<F, C, AuthorityId> where
    DigestItemFor<F::Block>: CompatibleDigestItem<F::Block, AuthorityId> + ProofDigestItem<F::Block> + ShardingDigestItem<u16>,
    C: Send + Sync,
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
        // get proof root from digest.
        let header_proof = header.digest().logs().iter().rev().filter_map(ProofDigestItem::as_xt_proof).next();
        if header_proof.is_none(){
            return Err("get proof log in header failed.".to_string());
        }
        let p_h = header_proof.unwrap();

        // check if header has a valid work proof
        let (pre_header, seal) = check_header::<F::Block, AuthorityId>(
            header.clone(),
            hash.clone(),
        )?;

        // TODO: verify body

        // check proof.
        let mut validate_proof = false;
        if let Some(proof) = proof.clone() {
            if let Ok(mlp) = MultiLayerProof::from_bytes(proof.as_slice()){
                // check proof root.
                let root = mlp.layer2_merkle.root();
                if root.as_ref() != p_h.as_slice() {
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
        let client = self.client.clone();
        if body.is_some() {
            let check_relay_extrinsic = move |exs: Vec<<F::Block as Block>::Extrinsic>| -> Result<(), String> {
                let err = Err("Block contains invalid extrinsic.".to_string());
                let api = client.runtime_api();
                let hash = hash.clone();
                let block_id = generic::BlockId::hash(hash);
                let tc = api.get_shard_count(&block_id).unwrap();    // total count
                let cs = api.get_curr_shard(&block_id).unwrap().unwrap();    // current shard

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
            let (root, proof) = gen_extrinsic_proof::<F::Block>(&header, &exs);
            if root.as_slice() != p_h.as_slice() {
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

    // TODO: check pow_target in seal
    // TODO: check shard_num, shard_count in header

    check_proof(&header, &seal)?;

    Ok((header, digest_item))
}
