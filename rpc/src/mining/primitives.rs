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
use yee_consensus_pow_primitives::PowTarget;
use runtime_primitives::traits::{Block as BlockT, Header as HeaderT, Digest as DigestT, ProvideRuntimeApi};
use parity_codec::{Decode, Encode};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::de::DeserializeOwned;
use yee_serde_hex::{SerdeHex, Hex};
use yee_runtime::opaque;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Job<Hash, Number: SerdeHex, AuthorityId> where
{
    pub hash: Hash,
    pub header: Header<Hash, Number>,
    pub digest_item: DigestItem<Hash, AuthorityId>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct JobResult<Hash> where
{
    pub hash: Hash,
    pub digest_item: ResultDigestItem<Hash>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Header<Hash, Number: SerdeHex> {
    pub parent_hash: Hash,
    #[serde(with = "SerdeHex")]
    pub number: Number,
    pub state_root: Hash,
    pub extrinsics_root: Hash,
    pub digest: Digest,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Digest {
    /// A list of logs in the digest.
    pub logs: Vec<Hex<Vec<u8>>>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct DigestItem<Hash, AuthorityId> {
    pub authority_id: AuthorityId,
    pub pow_target: PowTarget,
    #[serde(with = "SerdeHex")]
    pub timestamp: u64,
    pub work_proof: WorkProof<Hash>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ResultDigestItem<Hash> {
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
    pub merkle_proof: Vec<Hash>,
}

impl<B, AuthorityId> From<DefaultJob<B, AuthorityId>> for Job<B::Hash, <B::Header as HeaderT>::Number, AuthorityId> where
    B: BlockT,
    AuthorityId: Decode + Encode + Clone,
    <B::Header as HeaderT>::Number: SerdeHex,
{
    fn from(j: DefaultJob<B, AuthorityId>) -> Self {
        Self {
            hash: j.hash,
            header: j.header.into(),
            digest_item: j.digest_item.into(),
        }
    }
}

impl<H> From<H> for Header<H::Hash, H::Number> where
    H: HeaderT,
    H::Number: SerdeHex,
{
    fn from(h: H) -> Self {
        Self {
            parent_hash: h.parent_hash().clone(),
            number: h.number().clone(),
            state_root: h.state_root().clone(),
            extrinsics_root: h.extrinsics_root().clone(),
            digest: h.digest().to_owned().into(),
        }
    }
}

impl<D> From<D> for Digest where
    D: DigestT,
{
    fn from(d: D) -> Self {
        Self {
            logs: d.logs().iter().map(|x| Hex(x.encode())).collect(),
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
            pow_target: ps.pow_target,
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


impl<B> Into<DefaultWorkProof<B>> for WorkProof<B::Hash> where
    B: BlockT
{
    fn into(self) -> DefaultWorkProof<B> {
        match self{
            WorkProof::Unknown => DefaultWorkProof::Unknown,
            WorkProof::Nonce(t) => DefaultWorkProof::Nonce(t.into()),
            WorkProof::Multi(t) => DefaultWorkProof::Multi(t.into()),
        }
    }
}

impl Into<DefaultProofNonce> for ProofNonce {
    fn into(self) -> DefaultProofNonce {
        DefaultProofNonce{
            extra_data: self.extra_data,
            nonce: self.nonce,
        }
    }
}

impl<B> Into<DefaultProofMulti<B>> for ProofMulti<B::Hash> where
    B: BlockT
{
    fn into(self) -> DefaultProofMulti<B> {
        DefaultProofMulti{
            extra_data: self.extra_data,
            merkle_root: self.merkle_root,
            nonce: self.nonce,
            merkle_proof: self.merkle_proof,
        }
    }
}
