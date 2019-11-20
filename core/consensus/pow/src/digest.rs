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

//! POW chain digests
//!
//! Implements POW signature wrapped in block header DigestItem.

use runtime_primitives::{
    codec::{
        Decode, Encode,
    },
    generic::DigestItem,
    traits::{Header, Block, Digest, BlakeTwo256},
};
use std::alloc;
use std::iter::FromIterator;
use std::collections::hash_map::HashMap;
use pow_primitives::YEE_POW_ENGINE_ID;
use yee_sharding::ShardingDigestItem;
use merkle_light::hash::Algorithm;
use merkle_light::proof::Proof;
use merkle_light::merkle::MerkleTree;
use yee_srml_executive::decode::RelayTransfer;
use yee_runtime::{Call, BalancesCall, UncheckedExtrinsic};
use yee_sharding_primitives::utils::shard_num_for;
use yee_merkle::{ProofHash, ProofAlgorithm, MultiLayerProof};
use primitives::{Blake2Hasher, H256};
use hash_db::Hasher as BlakeHasher;
use ansi_term::Colour;
use log::info;
use super::PowSeal;

/// Digest item acts as a valid POW consensus digest.
pub trait CompatibleDigestItem<B: Block, AuthorityId: Decode + Encode + Clone>: Sized {
    /// construct digest item with work proof
    fn pow_seal(seal: PowSeal<B, AuthorityId>) -> Self;

    /// get work proof if digest item is pow item
    fn as_pow_seal(&self) -> Option<PowSeal<B, AuthorityId>>;
}

impl<B, Hash, AuthorityId, SealSignature> CompatibleDigestItem<B, AuthorityId> for DigestItem<Hash, AuthorityId, SealSignature> where
    B: Block,
    AuthorityId: Decode + Encode + Clone,
{
    fn pow_seal(seal: PowSeal<B, AuthorityId>) -> Self {
        DigestItem::Consensus(YEE_POW_ENGINE_ID, seal.encode())
    }

    fn as_pow_seal(&self) -> Option<PowSeal<B, AuthorityId>> {
        match self {
            DigestItem::Consensus(YEE_POW_ENGINE_ID, seal) => Decode::decode(&mut &seal[..]),
            _ => None
        }
    }
}

const XT_PROOF_PREFIX: [u8; 2] = [b'x', b't'];

/// Digest item for proof.
pub trait ProofDigestItem<B: Block>: Sized {
    /// gen extrinsic proof.
    fn gen_xt_proof(data: Vec<u8>) -> Self;
    /// revert to region data.
    fn as_xt_proof(&self) -> Option<Vec<u8>>;
}

impl<B, Hash, AuthorityId, SealSignature> ProofDigestItem<B> for DigestItem<Hash, AuthorityId, SealSignature> where
    B: Block,
{
    fn gen_xt_proof(data: Vec<u8>) -> Self {
        DigestItem::Other([XT_PROOF_PREFIX.to_vec(), data].concat())
    }

    fn as_xt_proof(&self) -> Option<Vec<u8>> {
        match self {
            DigestItem::Other(data) if data.len() == 34 && data[0] == b'x' && data[1] == b't'
            => {
                Some(data[2..].to_vec())
            }
            _ => None
        }
    }
}