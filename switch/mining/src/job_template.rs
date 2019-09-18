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
use std::convert::From;
use crate::job::Job;
use yee_serde_hex::SerdeHex;
pub type DifficultyType = primitives::U256;
pub type Hash = primitives::H256;

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct JobTemplate {
    pub difficulty: DifficultyType,
    pub rawHash:Hash,
    pub url:String,

}
impl JobTemplate {
    pub fn from_job(str:String,job:Job) -> Self {
        let Job {
             digest_item,
             hash,
             header,
        } = job;
        Self {
            difficulty:digest_item.difficulty,
            rawHash: hash,
            url: str
        }
    }
}


pub struct ProofNonce {
    /// Extra Data used to encode miner info AND more entropy
    pub extra_data: Vec<u8>,
    /// POW block nonce
    pub nonce: u64,
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
