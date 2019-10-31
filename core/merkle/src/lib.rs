use std::marker::PhantomData;
use std::default::Default;
use std::hash::Hasher;
use merkle_light::{
    merkle::MerkleTree,
    hash::Algorithm,
    proof::Proof,
};
use parity_codec::{Encode, Decode};
use runtime_primitives::traits::{Hash as HashT, BlakeTwo256};

#[derive(Debug, Clone)]
pub struct ProofAlgorithm<H: HashT>(Vec<u8>, PhantomData<H>);

impl<H: HashT> ProofAlgorithm<H> {
    fn new() -> ProofAlgorithm<H> {
        ProofAlgorithm(Vec::with_capacity(32), PhantomData)
    }
}

impl<H: HashT> Default for ProofAlgorithm<H> {
    fn default() -> ProofAlgorithm<H> {
        ProofAlgorithm::new()
    }
}

impl<H: HashT> Hasher for ProofAlgorithm<H> {
    #[inline]
    fn finish(&self) -> u64 {
        unimplemented!()
    }

    #[inline]
    fn write(&mut self, msg: &[u8]) {
        self.0.extend_from_slice(msg)
    }
}

pub type ProofHash<H> = <H as HashT>::Output;

impl<H: HashT> Algorithm<ProofHash<H>> for ProofAlgorithm<H> {
    #[inline]
    fn hash(&mut self) -> ProofHash<H> {
        H::hash(&self.0)
    }

    #[inline]
    fn reset(&mut self) {
        self.0.truncate(0);
    }

    fn leaf(&mut self, leaf: ProofHash<H>) -> ProofHash<H> {
        leaf
    }

    fn node(&mut self, left: ProofHash<H>, right: ProofHash<H>) -> ProofHash<H> {
        self.write(left.as_ref());
        self.write(right.as_ref());
        self.hash()
    }
}

#[derive(Debug, Default, Clone, Encode, Decode)]
pub struct MultiLayerProof {
    layer2_merkle: Option<MerkleTree<ProofHash<BlakeTwo256>, ProofAlgorithm<BlakeTwo256>>>,
    layer2_proof: Vec<u8>,
    layer1_merkles: Vec<(u16, Option<MerkleTree<ProofHash<BlakeTwo256>, ProofAlgorithm<BlakeTwo256>>>)>,
}

impl MultiLayerProof {
    /// construct a object
    pub fn new(layer1_merkles: Vec<(u16, Option<MerkleTree<ProofHash<BlakeTwo256>, ProofAlgorithm<BlakeTwo256>>>)>,
               layer2_merkle: Option<MerkleTree<ProofHash<BlakeTwo256>, ProofAlgorithm<BlakeTwo256>>>,
               layer2_proof: Vec<u8>) -> Self {
        Self {
            layer2_merkle,
            layer2_proof,
            layer1_merkles,
        }
    }

    /// Gen proof for special shard_num.
    pub fn gen_proof(&self, shard_num: u16) -> Self {
//        let shard_num = shard_num as usize;
        let mut result = Default::default();
        if self.layer2_merkle.is_none() {
            return result;
        }
        let layer2 = &(self.layer2_merkle.clone().unwrap());
        if self.layer1_merkles.len() != layer2.leafs() || shard_num as usize >= layer2.leafs() {
            return result;
        }
        result.layer2_proof = layer2.gen_proof(shard_num as usize).into_bytes();
        let layer1 = self.layer1_merkles.get(shard_num as usize);
        if let Some((num, data)) = layer1 {
            result.layer1_merkles = vec![(*num, (*data).clone())];
        }
        result
    }

    /// Turns a MultiLayerProof into the raw bytes.
    pub fn into_bytes(&self) -> Vec<u8> {
        self.encode()
    }

    /// Tries to parse `bytes` into MultiLayerProof.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ()> {
        Decode::decode(&mut &bytes[..]).ok_or(())
    }
}