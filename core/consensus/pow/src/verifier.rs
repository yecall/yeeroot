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
    foreign_chain::{ForeignChain},
    foreign_chain_interface::foreign_chains::ForeignChains,
};
use super::CompatibleDigestItem;
use crate::pow::check_proof;
use yee_sharding::ShardingDigestItem;
use merkle_light::proof::Proof as MLProof;
use merkle_light::merkle::MerkleTree;
use yee_merkle::{ProofHash, ProofAlgorithm, MultiLayerProof};
use ansi_term::Colour;
use log::{debug, warn};
use parking_lot::RwLock;

/// Verifier for POW blocks.
pub struct PowVerifier<F: ServiceFactory, C, AuthorityId> {
    pub client: Arc<C>,
    pub inherent_data_providers: InherentDataProviders,
    pub foreign_chains: Arc<RwLock<Option<ForeignChain<F, C>>>>,
    pub phantom: PhantomData<AuthorityId>,
}

#[forbid(deprecated)]
impl<F, B, C, AuthorityId> Verifier<B> for PowVerifier<F, C, AuthorityId> where
    B: Block,
    DigestItemFor<B>: CompatibleDigestItem<B, AuthorityId> + ShardingDigestItem<u16>,
    C: Send + Sync,
    AuthorityId: Decode + Encode + Clone + Send + Sync,
    F: ServiceFactory + Send + Sync,
    <F as ServiceFactory>::Configuration: Send + Sync,
    C : ProvideRuntimeApi,
    C : HeaderBackend<<F as ServiceFactory>::Block>,
    C : BlockBody<<F as ServiceFactory>::Block>,
    C : BlockchainEvents<<F as ServiceFactory>::Block>,
    C : ChainHead<<F as ServiceFactory>::Block>,
    <C as ProvideRuntimeApi>::Api: ShardingAPI<<F as ServiceFactory>::Block>,
{
    fn verify(
        &self,
        origin: BlockOrigin,
        header: <B as Block>::Header,
        justification: Option<Justification>,
        proof: Option<Proof>,
        body: Option<Vec<<B as Block>::Extrinsic>>,
    ) -> Result<(ImportBlock<B>, Option<Vec<AuthorityIdFor<B>>>), String> {
        let number = header.number().clone();
        let hash = header.hash();
        let _parent_hash = *header.parent_hash();

        // check if header has a valid work proof
        let (pre_header, seal) = check_header::<B, AuthorityId>(
            header,
            hash.clone(),
        )?;

        // TODO: verify body

        // check proof.
        let mut validate_proof = false;
        if let Some(proof) = proof.clone() {
            if let Ok(mlp) = MultiLayerProof::from_bytes(proof.as_slice()){
                if let Ok(mt_proof) = MLProof::from_bytes(mlp.layer2_proof.as_slice()) {
                    let mt_proof: MLProof<ProofHash<BlakeTwo256>> = mt_proof;
                    if mt_proof.validate::<ProofAlgorithm<BlakeTwo256>>() {
                        validate_proof = true;
                    }
                }
            }
        }
        if !validate_proof{
            warn!("{}, number:{}, hash:{}", Colour::Red.paint("Proof validate failed"), number, hash.clone());
            return Err("Proof validate failed.".to_string());
        } else{
            debug!("{}, number:{}, hash:{}", Colour::Green.paint("Proof validated"), number, hash.clone());
        }
        if body.is_some() {
            let exs = body.clone().unwrap();
            // check relay extrinsic.
//            let check_relay = self.check_relay_extrinsic(exs);
//            if check_relay.is_err() {
//                return Err(check_relay.err().unwrap());
//            }
        }

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

impl<F, B, C, AuthorityId> CheckRelay<B> for PowVerifier<F, C, AuthorityId> where
    B: Block,
    DigestItemFor<B>: CompatibleDigestItem<B, AuthorityId> + ShardingDigestItem<u16>,
    C: Send + Sync,
    AuthorityId: Decode + Encode + Clone + Send + Sync,
    F: ServiceFactory,
    C : ProvideRuntimeApi,
    C : HeaderBackend<<F as ServiceFactory>::Block>,
    C : BlockBody<<F as ServiceFactory>::Block>,
    C : BlockchainEvents<<F as ServiceFactory>::Block>,
    C : ChainHead<<F as ServiceFactory>::Block>,
    <C as ProvideRuntimeApi>::Api: ShardingAPI<<F as ServiceFactory>::Block>,{
    fn check_relay_extrinsic(&self, body: Vec<<B as Block>::Extrinsic>) -> Result<(), String> {
        // todo
        for tx in body {
//            let rt = RelayTransfer::decode(tx.encode());
//            if rt.is_some() {
//                let rt = rt.unwrap();
//                let h = rt.hash();
//            }
        }

        Ok(())
    }
}

trait CheckRelay<B> where B: Block {
    fn check_relay_extrinsic(&self, body: Vec<<B as Block>::Extrinsic>) -> Result<(), String>;
}