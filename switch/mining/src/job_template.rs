

use serde_derive::{Deserialize, Serialize};
use std::convert::From;
use yee_merkle::proof::Proof;
use crate::job::Job;

pub type DifficultyType = primitives::U256;

pub type Hash = primitives::H256;


#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct JobTemplate {
    pub difficulty: DifficultyType,
    pub rawHash:Hash,

}
impl JobTemplate {
    pub fn from_job(job:Job) -> Self {
        let Job {
             digest_item,
             hash,
             header,
        } = job;
        Self {
            difficulty:digest_item.difficulty,
            rawHash: hash
        }
    }
}


pub struct ProofNonce {
    /// Extra Data used to encode miner info AND more entropy
    pub extra_data: Vec<u8>,
    /// POW block nonce
    pub nonce: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ProofMulti {
    /// Extra Data used to encode miner info AND more entropy
    pub extra_data: Vec<u8>,
    /// merkle root of multi-mining headers
    pub merkle_root: Hash,
    /// merkle tree spv proof
    pub merkle_proof: Proof<[u8;32]>,
    /// POW block nonce
    pub nonce: u64,
    /// shard info
    pub shard_num: u32,
    pub shard_cnt: u32,

}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Task {
    pub work_id: String,
    /// Extra Data used to encode miner info AND more entropy
    pub extra_data: Vec<u8>,
    /// merkle root of multi-mining headers
    pub merkle_root: Hash,


}
