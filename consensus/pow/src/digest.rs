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

//! POW chain digests
//!
//! Implements POW signature wrapped in block header DigestItem.

use parity_codec::{Decode, Encode};
use runtime_primitives::{generic::DigestItem};

use pow_primitives::YEE_POW_ENGINE_ID;
use super::{pow::WorkProof};

/// Digest item acts as a valid POW consensus digest.
pub trait CompatibleDigestItem: Sized {
    /// construct digest item with work proof
    fn pow_seal(proof: WorkProof) -> Self;

    /// get work proof if digest item is pow item
    fn as_pow_seal(&self) -> Option<WorkProof>;
}

impl<Hash, AuthorityId, SealSignature> CompatibleDigestItem for DigestItem<Hash, AuthorityId, SealSignature> {
    fn pow_seal(proof: WorkProof) -> Self {
        DigestItem::Consensus(YEE_POW_ENGINE_ID, proof.encode())
    }

    fn as_pow_seal(&self) -> Option<WorkProof> {
        match self {
            DigestItem::Consensus(YEE_POW_ENGINE_ID, seal) => Decode::decode(&mut &seal[..]),
            _ => None
        }
    }
}
