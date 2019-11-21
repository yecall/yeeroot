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
use serde_derive::{Deserialize, Serialize};
use yee_serde_hex::SerdeHex;

pub type PowTarget = primitives::U256;
pub type Hash = primitives::H256;

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct JobTemplate {
    pub pow_target: PowTarget,
    pub raw_hash: Hash,
    pub url: String,

}

impl JobTemplate {
    pub fn from_job(str: String, job: Job) -> Self {
        let Job {
            digest_item,
            hash,
            header,
        } = job;
        Self {
            pow_target: digest_item.pow_target,
            raw_hash: hash,
            url: str,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct ProofMulti {
    /// Extra Data used to encode miner info AND more entropy
    #[serde(with = "SerdeHex")]
    pub extra_data: Vec<u8>,
    /// merkle root of multi-mining headers
    pub merkle_root: Hash,
    /// merkle tree spv proof
    pub merkle_proof: Vec<Hash>,
    /// POW block nonce
    #[serde(with = "SerdeHex")]
    pub nonce: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Task {
    pub work_id: String,
    /// Extra Data used to encode miner info AND more entropy
    pub extra_data: Vec<u8>,
    /// merkle root of multi-mining headers
    pub merkle_root: Hash,

}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct Job {
    pub digest_item: DigestItem,
    pub hash: Hash,
    pub header: Header,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct DigestItem {
    pub authority_id: String,
    pub pow_target: PowTarget,
    pub timestamp: String,
    pub work_proof: String,
}

#[allow(non_snake_case)]
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct Header {
    pub digest: Digest,
    pub extrinsicsRoot: Hash,
    pub number: String,
    pub parentHash: Hash,
    pub stateRoot: Hash,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct Digest {
    pub logs: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct JobResult
{
    pub hash: Hash,
    pub digest_item: ResultDigestItem,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct ResultDigestItem {
    pub work_proof: WorkProof,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub enum WorkProof {
    Unknown,
    Nonce(ProofNonce),
    Multi(ProofMulti),
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct ProofNonce {
    pub extra_data: Vec<u8>,
    pub nonce: u64,
}

