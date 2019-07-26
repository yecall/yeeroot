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

use runtime_primitives::{
    codec::{
        Decode, Encode,
    },
    generic::DigestItem,
};

use pow_primitives::YEE_POW_ENGINE_ID;
use super::PowSeal;

/// Digest item acts as a valid POW consensus digest.
pub trait CompatibleDigestItem<AccountId: Decode + Encode>: Sized {
    /// construct digest item with work proof
    fn pow_seal(seal: PowSeal<AccountId>) -> Self;

    /// get work proof if digest item is pow item
    fn as_pow_seal(&self) -> Option<PowSeal<AccountId>>;
}

impl<Hash, AuthorityId, SealSignature, AccountId> CompatibleDigestItem<AccountId> for DigestItem<Hash, AuthorityId, SealSignature> where
    AccountId: Decode + Encode,
{
    fn pow_seal(seal: PowSeal<AccountId>) -> Self {
        DigestItem::Consensus(YEE_POW_ENGINE_ID, seal.encode())
    }

    fn as_pow_seal(&self) -> Option<PowSeal<AccountId>> {
        match self {
            DigestItem::Consensus(YEE_POW_ENGINE_ID, seal) => Decode::decode(&mut &seal[..]),
            _ => None
        }
    }
}
