//

use rstd::prelude::*;

use runtime_primitives::{
    codec::{
        Decode, Encode,
        Input, Output,
    },
};

#[cfg(feature = "std")]
use serde::Serialize;

/// POW data used in block header
#[derive(PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(Debug, Serialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "std", serde(deny_unknown_fields))]
pub enum WorkProof {
    Unknown,
    FixedNonce(ProofFixedNonce),
}

/// Referencing view for WorkProof
pub enum WorkProofRef<'a> {
    Unknown,
    FixedNonce(&'a ProofFixedNonce),
}

/// Type ID of WorkProof used for encoding/decoding
#[derive(Encode, Decode)]
enum WorkProofType {
    Unknown = 0,
    FixedNonce = 1,
}

impl WorkProof {
    fn wp_ref(&self) -> WorkProofRef {
        match *self {
            WorkProof::Unknown => WorkProofRef::Unknown,
            WorkProof::FixedNonce(ref v) => WorkProofRef::FixedNonce(v),
        }
    }
}

impl<'a> Encode for WorkProofRef<'a> {
    fn encode_to<T: Output>(&self, dest: &mut T) {
        match *self {
            WorkProofRef::Unknown => {
                WorkProofType::Unknown.encode_to(dest);
            }
            WorkProofRef::FixedNonce(v) => {
                WorkProofType::FixedNonce.encode_to(dest);
                v.encode_to(dest);
            }
        }
    }
}

impl Decode for WorkProof {
    fn decode<I: Input>(value: &mut I) -> Option<Self> {
        let proof_type: WorkProofType = Decode::decode(value)?;
        match proof_type {
            WorkProofType::Unknown => Some(WorkProof::Unknown),
            WorkProofType::FixedNonce => Some(WorkProof::FixedNonce(
                Decode::decode(value)?,
            )),
        }
    }
}

impl Encode for WorkProof {
    fn encode(&self) -> Vec<u8> {
        self.wp_ref().encode()
    }
}

/// Classical pow proof with extra data entropy and 64b nonce
#[derive(PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(Debug, Serialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "std", serde(deny_unknown_fields))]
pub struct ProofFixedNonce {
    /// Extra Data used to encode miner info AND more entropy
    pub extra_data: Vec<u8>,
    /// POW block nonce
    pub nonce: u64,
}

impl Decode for ProofFixedNonce {
    fn decode<I: Input>(value: &mut I) -> Option<Self> {
        Some(ProofFixedNonce {
            extra_data: Decode::decode(value)?,
            nonce: Decode::decode(value)?,
        })
    }
}

impl Encode for ProofFixedNonce {
    fn encode_to<T: Output>(&self, dest: &mut T) {
        dest.push(&self.extra_data);
        dest.push(&self.nonce);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn work_proof_can_encode() {}
}
