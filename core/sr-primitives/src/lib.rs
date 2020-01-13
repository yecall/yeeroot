#![cfg_attr(not(feature = "std"), no_std)]

use parity_codec::{Encode, Decode, Input, Compact};
use rstd::prelude::*;
use substrate_primitives::{Blake2Hasher, Hasher, H256};

#[derive(PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct RelayParams<Hash> where
    Hash: Decode + Clone
{
    origin: Vec<u8>,
    number: Compact<u64>,
    hash: Hash,
    block_hash: Hash,
    parent_hash: Hash
}

const MIN_RELAY_SIZE: usize = 2 + 32 + 32 + 64;

impl<Hash> RelayParams<Hash>  where
    Hash: Decode + Clone + Default
{
//    pub fn new(origin: Vec<u8>, number: Compact<u64>, hash: Hash, block_hash: Hash, parent_hash: Hash) -> Self {
//        RelayParams{origin, number, hash, block_hash, parent_hash}
//    }

    pub fn origin(self) -> Vec<u8> {
        self.origin
    }

    pub fn number(&self) -> Compact<u64> {
        self.number.clone()
    }

    pub fn hash(&self) -> Hash {
        self.hash.clone()
    }

    pub fn block_hash(&self) -> Hash {
        self.block_hash.clone()
    }

    pub fn parent_hash(&self) -> Hash {
        self.parent_hash.clone()
    }

    /// decode from input
    pub fn decode(input: Vec<u8>) -> Option<Self> {
        let mut input = input.as_slice();
        if input.len() <= MIN_RELAY_SIZE {
            return None
        }
        // length
        let _len: Vec<()> = match Decode::decode(&mut input) {
            Some(len) => len,
            None => return None
        };
        // version
        let version = match input.read_byte() {
            Some(v) => v,
            None => return None
        };
        // is signed
        let is_signed = version & 0b1000_0000 != 0;
        let version = version & 0b0111_1111;
        // has signed or version not satisfy
        if is_signed || version != 1u8 {
            return None
        }
        // module
        let _module: u8 = match input.read_byte() {
            Some(m) => m,
            None => return None
        };
        // function
        let _func: u8 = match input.read_byte() {
            Some(f) => f,
            None => return None
        };
        // origin transfer
        let origin: Vec<u8> = match Decode::decode(&mut input) {
            Some(ot) => ot,
            None => return None
        };
        // which block's number the origin transfer in
        let number: Compact<u64> = match Decode::decode(&mut input) {
            Some(h) => h,
            None => return None
        };
        // block hash
        let block_hash: Hash = match Decode::decode(&mut input) {
            Some(h) => h,
            None => return None
        };
        // which block's parent hash the origin transfer in
        let parent_hash: Hash = match Decode::decode(&mut input) {
            Some(h) => h,
            None => return None
        };
        let hash = Decode::decode(&mut Blake2Hasher::hash(origin.as_slice()).encode().as_slice()).unwrap();
        //Blake2Hasher::hash(origin.as_slice()).into();
        Some(Self{origin, number, hash, block_hash, parent_hash})
    }

}

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum RelayTypes {
    Balance,
    Assets
}