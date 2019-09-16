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
        Decode, Encode,
    },
    traits::Block,
};
use {
    pow_primitives::DifficultyType,
};

/// Max length in bytes for pow extra data
pub const MAX_EXTRA_DATA_LENGTH: usize = 32;

/// POW consensus seal
#[derive(Clone, Debug, Decode, Encode)]
pub struct PowSeal<B: Block, AuthorityId: Decode + Encode> {
    pub authority_id: AuthorityId,
    pub difficulty: DifficultyType,
    pub timestamp: u64,
    pub work_proof: WorkProof<B>,
}

/// POW proof used in block header
#[derive(Clone, Debug)]
#[derive(Decode, Encode)]
pub enum WorkProof<B: Block> {
    #[codec(index = "0")]
    Unknown,
    #[codec(index = "1")]
    Nonce(ProofNonce),
    #[codec(index = "2")]
    Multi(ProofMulti<B>),
}

/// Classical pow proof with extra data entropy and 64b nonce
#[derive(Clone, Debug)]
#[derive(Decode, Encode)]
pub struct ProofNonce {
    /// Extra Data used to encode miner info AND more entropy
    pub extra_data: Vec<u8>,
    /// POW block nonce
    pub nonce: u64,
}

impl ProofNonce {
    pub fn get_with_prefix_len(prefix: &str, extra_bytes: usize, nonce: u64) -> Self {
        let len = prefix.len() + extra_bytes;
        assert!(len <= MAX_EXTRA_DATA_LENGTH);

        let mut extra_data = prefix.as_bytes().to_vec();
        extra_data.resize_with(len, Default::default);
        Self {
            extra_data,
            nonce,
        }
    }
}

/// Multi-Mining pow proof with header-trie spv proof
#[derive(Clone, Debug)]
#[derive(Decode, Encode)]
pub struct ProofMulti<B: Block> {
    /// Extra Data used to encode miner info AND more entropy
    pub extra_data: Vec<u8>,
    /// merkle root of multi-mining headers
    pub merkle_root: B::Hash,
    /// POW block nonce
    pub nonce: u64,
    /// merkle tree spv proof
    pub merkle_proof: Vec<u8>,
}

impl<B: Block> ProofMulti<B> {
    //
}

impl<B, AccountId> PowSeal<B, AccountId> where
    B: Block,
    AccountId: Decode + Encode,
{
    pub fn check_seal(&self, hash: B::Hash) -> Result<(), String> {
        match self.work_proof {
            WorkProof::Unknown => Err(format!("invalid work proof")),
            WorkProof::Nonce(ref proof_nonce) => {
                if proof_nonce.extra_data.len() > MAX_EXTRA_DATA_LENGTH {
                    return Err(format!("extra data too long"));
                }
                let proof_difficulty = DifficultyType::from(hash.as_ref());
                if proof_difficulty > self.difficulty {
                    return Err(format!("difficulty not enough, need {}, got {}", self.difficulty, proof_difficulty));
                }
                Ok(())
            }
            WorkProof::Multi(ref _proof_multi) => {
                Err(format!("TODO"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use yee_runtime::Block;

    #[test]
    fn proof_nonce_encode() {
        let tests = vec![
            ("Test Extra", 0x10001, hex!("01 28 54657374204578747261 0100 0100 0000 0000").to_vec()),
            ("YeeRoot", 0xABCD000098760000, hex!("01 1C 596565526f6f74 0000 7698 0000 CDAB").to_vec()),
            ("Yee Root", 0xABCD000098765432, hex!("01 20 59656520526f6f74 3254 7698 0000 CDAB").to_vec()),
        ];

        for (extra, nonce, expected) in tests {
            let proof = WorkProof::<Block>::Nonce(ProofNonce { extra_data: extra.as_bytes().to_vec(), nonce });
            assert_eq!(proof.encode(), expected);
        }
    }
}
