

use primitives::{H256,U256};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::de::DeserializeOwned;

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

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct ProofMulti {
    pub extra_data: String,
    pub merkle_root: H256,
    pub nonce: String,
    pub merkle_proof: String,
}
