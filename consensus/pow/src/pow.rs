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

//! POW work proof used in block header digest

use runtime_primitives::{
    codec::{
        Decode, Encode, Input, Output,
    }
};

/// POW proof used in block header
pub enum WorkProof {
    Unknown,
    Nonce(ProofNonce),
}

/// Referencing view for WorkProof
pub enum WorkProofRef<'a> {
    Unknown,
    Nonce(&'a ProofNonce),
}

/// Type ID of WorkProof used for encoding/decoding
#[repr(u32)]
#[derive(Decode, Encode)]
enum WorkProofType {
    Unknown = 0,
    Nonce = 1,
}

impl WorkProof {
    fn wp_ref(&self) -> WorkProofRef {
        match *self {
            WorkProof::Unknown => WorkProofRef::Unknown,
            WorkProof::Nonce(ref v) => WorkProofRef::Nonce(v),
        }
    }
}

impl<'a> Encode for WorkProofRef<'a> {
    fn encode_to<T: Output>(&self, dest: &mut T) {
        match *self {
            WorkProofRef::Unknown => {
                WorkProofType::Unknown.encode_to(dest);
            }
            WorkProofRef::Nonce(v) => {
                WorkProofType::Nonce.encode_to(dest);
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
            WorkProofType::Nonce => Some(WorkProof::Nonce(
                Decode::decode(value)?,
            )),
        }
    }
}

impl Encode for WorkProof {
    fn encode_to<T: Output>(&self, dest: &mut T) {
        self.wp_ref().encode_to(dest)
    }
}

/// Classical pow proof with extra data entropy and 64b nonce
pub struct ProofNonce {
    /// Extra Data used to encode miner info AND more entropy
    pub extra_data: Vec<u8>,
    /// POW block nonce
    pub nonce: u64,
}

impl Decode for ProofNonce {
    fn decode<I: Input>(value: &mut I) -> Option<Self> {
        Some(ProofNonce {
            extra_data: Decode::decode(value)?,
            nonce: Decode::decode(value)?,
        })
    }
}

impl Encode for ProofNonce {
    fn encode_to<T: Output>(&self, dest: &mut T) {
        dest.push(&self.extra_data);
        dest.push(&self.nonce);
    }
}
