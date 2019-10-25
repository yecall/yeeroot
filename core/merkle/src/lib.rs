use std::marker::PhantomData;
use std::default::Default;
use std::hash::Hasher;
use merkle_light::{
    hash::Algorithm,
    // proof::Proof,
};
use parity_codec::{Encode, Decode};
use runtime_primitives::traits::{
    Hash as HashT,
};

#[derive(Clone)]
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
//impl<H: HashT> Algorithm<H> for ProofAlgorithm<H> where H: Encode+Decode {
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