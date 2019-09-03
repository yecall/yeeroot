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

use yee_consensus_pow::{JobManager, DefaultJob, PowSeal,
                        WorkProof as DefaultWorkProof, ProofNonce as DefaultProofNonce, ProofMulti as DefaultProofMulti};
use yee_consensus_pow_primitives::DifficultyType;
use runtime_primitives::traits::{Block as BlockT, ProvideRuntimeApi};
use parity_codec::{Decode, Encode};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::de::DeserializeOwned;
use serde_hex::{SerHex, SerHexSeq, StrictPfx, CompactPfx};
use super::serde_hex::SerdeHex;

#[derive(Clone, Serialize)]
pub struct Job<Hash, Header, AuthorityId> where
{
    pub hash: Hash,
    pub header: Header,
    pub digest_item: DigestItem<Hash, AuthorityId>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct JobResult<Hash, AuthorityId> where
{
    pub hash: Hash,
    pub digest_item: DigestItem<Hash, AuthorityId>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct DigestItem<Hash, AuthorityId> {
    pub authority_id: AuthorityId,
    pub difficulty: DifficultyType,
    #[serde(with = "SerdeHex")]
    pub timestamp: u64,
    pub work_proof: WorkProof<Hash>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum WorkProof<Hash> {
    Unknown,
    Nonce(ProofNonce),
    Multi(ProofMulti<Hash>),
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ProofNonce {
    #[serde(with = "SerdeHex")]
    pub extra_data: Vec<u8>,
    #[serde(with = "SerdeHex")]
    pub nonce: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ProofMulti<Hash> {
    #[serde(with = "SerdeHex")]
    pub extra_data: Vec<u8>,
    pub merkle_root: Hash,
    #[serde(with = "SerdeHex")]
    pub nonce: u64,
    #[serde(with = "SerdeHex")]
    pub merkle_proof: Vec<u8>,
}

impl<B, AuthorityId> From<DefaultJob<B, AuthorityId>> for Job<B::Hash, B::Header, AuthorityId> where
    B: BlockT,
    AuthorityId: Decode + Encode + Clone
{
    fn from(j: DefaultJob<B, AuthorityId>) -> Self {
        Self {
            hash: j.hash,
            header: j.header,
            digest_item: j.digest_item.into(),
        }
    }
}

impl<B, AuthorityId> From<PowSeal<B, AuthorityId>> for DigestItem<B::Hash, AuthorityId> where
    B: BlockT,
    AuthorityId: Decode + Encode + Clone
{
    fn from(ps: PowSeal<B, AuthorityId>) -> Self {
        Self {
            authority_id: ps.authority_id,
            difficulty: ps.difficulty,
            timestamp: ps.timestamp,
            work_proof: ps.work_proof.into(),
        }
    }
}

impl<B> From<DefaultWorkProof<B>> for WorkProof<B::Hash> where
    B: BlockT
{
    fn from(dwp: DefaultWorkProof<B>) -> Self {
        match dwp{
            DefaultWorkProof::Unknown => WorkProof::Unknown,
            DefaultWorkProof::Nonce(t) => WorkProof::Nonce(t.into()),
            DefaultWorkProof::Multi(t) => WorkProof::Multi(t.into()),
        }
    }
}

impl From<DefaultProofNonce> for ProofNonce {
    fn from(dpn: DefaultProofNonce) -> Self {
        Self{
            extra_data: dpn.extra_data,
            nonce: dpn.nonce,
        }
    }
}

impl<B> From<DefaultProofMulti<B>> for ProofMulti<B::Hash> where
    B: BlockT
{
    fn from(dpm: DefaultProofMulti<B>) -> Self {
        Self{
            extra_data: dpm.extra_data,
            merkle_root: dpm.merkle_root,
            nonce: dpm.nonce,
            merkle_proof: dpm.merkle_proof,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mining::primitives::{JobResult, DigestItem, WorkProof};
    use substrate_primitives::sr25519::Public;
    use runtime_primitives::traits::BlakeTwo256;
    use substrate_primitives::{H256, U256};

    #[test]
    fn test_log1(){

        let s: JobResult<H256, Public> = serde_json::from_str(r#"{	"digest_item": {      "authority_id": "5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu",      "difficulty": "0x141d60fdeba093869f5954234b8342c79970d5ba5d2846fbb3c25eb2d9514",      "timestamp": "0x16cf22f502c",      "work_proof": {        "Nonce": {          "extra_data": "0x010203040506",          "nonce": "0x400"        }      }    },    "hash": "0x2f9f279997f6da548ad0aebddf3d8f6ff5e5ff292042f1df4b23cd406f486c40"}"#).unwrap();

        assert_eq!(s, JobResult{
            hash: H256::from([0; 32]),
            digest_item : DigestItem {
                authority_id: Public([0; 32]),
                difficulty: U256::from(0u128),
                timestamp : 0,
                work_proof: WorkProof::Unknown,
            }
        })
    }

}