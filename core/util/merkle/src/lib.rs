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
use log::error;

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
    pub layer2_merkle: Option<MerkleTree<ProofHash<BlakeTwo256>, ProofAlgorithm<BlakeTwo256>>>,
    pub layer2_proof: Option<Vec<u8>>,
    layer1_merkles: Vec<(u16, Option<MerkleTree<ProofHash<BlakeTwo256>, ProofAlgorithm<BlakeTwo256>>>)>,
}

impl MultiLayerProof {
    /// construct a object with layer-2.
    pub fn new_with_layer2(layer2_merkle: MerkleTree<ProofHash<BlakeTwo256>, ProofAlgorithm<BlakeTwo256>>,
                           layer1_merkles: Vec<(u16, Option<MerkleTree<ProofHash<BlakeTwo256>, ProofAlgorithm<BlakeTwo256>>>)>) -> Self {
        Self {
            layer2_merkle: Some(layer2_merkle),
            layer2_proof: None,
            layer1_merkles,
        }
    }

    /// construct a object with proof.
    pub fn new_with_poof(layer2_proof: Vec<u8>, layer1_merkles: Vec<(u16, Option<MerkleTree<ProofHash<BlakeTwo256>, ProofAlgorithm<BlakeTwo256>>>)>) -> Self {
        Self {
            layer2_merkle: None,
            layer2_proof: Some(layer2_proof),
            layer1_merkles,
        }
    }

    /// Gen proof for special shard_num.
    pub fn gen_proof(&self, shard_num: u16) -> Option<Self> {
        if self.layer2_merkle.is_none() {
            return None;
        }
        let tree = self.layer2_merkle.as_ref().unwrap();
        let shard_num = shard_num as usize;
        if self.layer1_merkles.len() != tree.leafs() || shard_num >= tree.leafs() {
            return None;
        }
        let layer1 = self.layer1_merkles.get(shard_num);
        let (num, data) = match layer1 {
            Some((num, data)) => (num, data),
            None => {
                error!("failed to get special layer1 merkle tree");
                return None;
            }
        };
        Some(MultiLayerProof::new_with_poof(tree.gen_proof(shard_num).into_bytes(), vec![(*num, (*data).clone())]))
    }

    /// Turns a MultiLayerProof into the raw bytes.
    pub fn into_bytes(&self) -> Vec<u8> {
        self.encode()
    }

    /// Tries to parse `bytes` into MultiLayerProof.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ()> {
        Decode::decode(&mut &bytes[..]).ok_or(())
    }

    /// Layer two merkle root.
    pub fn layer2_root(&self) -> Option<ProofHash<BlakeTwo256>> {
        if let Some(tree) = self.layer2_merkle.as_ref() {
            return Some(tree.root());
        }
        match self.layer2_proof.as_ref() {
            Some(proof)=>{
                // check proof self.
                if let Ok(mt_proof) = Proof::from_bytes(proof.as_slice()) {
                    let mt_proof: Proof<ProofHash<BlakeTwo256>> = mt_proof;
                     return Some(mt_proof.root());
                }
            },
            None => {},
        }
        None
    }

    /// Tries contains
    pub fn contains(&self, shard: u16, hash: ProofHash<BlakeTwo256>) -> bool {
        for (num, tr) in &self.layer1_merkles {
            if *num == shard {
                if tr.is_some() {
                    return tr.as_ref().unwrap().contains(hash);
                }
            }
        }
        return false;
    }
}