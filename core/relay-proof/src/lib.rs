use runtime_primitives::{
    generic::DigestItem,
    traits::Block,
};
use parity_codec::{Encode, Decode};
use primitives::H256;
use std::slice;

pub const PROOF_MODULE_LOG_PREFIX: u8 = 4;

/// Digest item for proof.
pub trait ProofDigestItem<B: Block>: Sized {
    /// gen extrinsic proof.
    fn gen_xt_proof(data: H256) -> Self;
    /// revert to region data.
    fn as_xt_proof(&self) -> Option<H256>;
}

impl<B, Hash, AuthorityId, SealSignature> ProofDigestItem<B> for DigestItem<Hash, AuthorityId, SealSignature> where
    B: Block,
{
    fn gen_xt_proof(data: H256) -> Self {
        let prefix: [u8; 2] = [PROOF_MODULE_LOG_PREFIX, 0];
        let data = Encode::encode(&(prefix, data));
        DigestItem::Other(data)
    }

    fn as_xt_proof(&self) -> Option<H256> {
        match self {
            DigestItem::Other(data) if data.len() >= 34 && data[0] == PROOF_MODULE_LOG_PREFIX && data[1] == 0
            => {
                let input = &mut &data[2..];
                let root = Decode::decode(input)?;
                Some(root)
            }
            _ => None
        }
    }
}