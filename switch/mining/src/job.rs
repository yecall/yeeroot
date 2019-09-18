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

use primitives::{H256,U256};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::de::DeserializeOwned;
use crate::job_template::ProofMulti;
pub type DifficultyType = primitives::U256;


#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct Job {
    pub digest_item: DigestItem,
    pub hash: H256,
    pub header:Header,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct DigestItem {
    pub authority_id: String,
    pub difficulty: DifficultyType,
    pub timestamp: String,
    pub work_proof: String,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]

pub struct Header {
    pub digest: Digest,
    pub extrinsicsRoot: H256,
    pub number: String,
    pub parentHash:H256,
    pub stateRoot:H256,
}
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]

pub struct Digest {
    pub logs:Vec<String>,
}



#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]

pub struct JobResult
{
    pub hash: H256,
    pub digest_item: ResultDigestItem,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct ResultDigestItem {
    pub work_proof: WorkProof,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub enum WorkProof{
    Unknown,
    Nonce(ProofNonce),
    Multi(ProofMulti),
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct ProofNonce {
    pub extra_data: Vec<u8>,
    pub nonce: u64,
}


