

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
