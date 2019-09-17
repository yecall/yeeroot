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

extern crate crypto;
use std::fmt;
use std::hash::Hasher;
use std::iter::FromIterator;
use yee_merkle::hash::{Algorithm, Hashable};
use yee_merkle::merkle::MerkleTree;
use crypto::sha2::Sha256;
use crypto::digest::Digest;
use primitives::H256;
use primitives::blake2_256;


#[derive(Clone)]
pub struct CryptoYeeAlgorithm(Sha256);

impl CryptoYeeAlgorithm {
   pub fn new() -> CryptoYeeAlgorithm {
        CryptoYeeAlgorithm(Sha256::new())
    }
}

impl Default for CryptoYeeAlgorithm {
   fn default() -> CryptoYeeAlgorithm {
        CryptoYeeAlgorithm::new()
    }
}

impl Hasher for CryptoYeeAlgorithm {
    #[inline]
    fn write(&mut self, msg: &[u8]) {
        self.0.input(msg)
    }

    #[inline]
    fn finish(&self) -> u64 {
        //unimplemented!()
        0
    }
}

pub type CryptoSHA256Hash = [u8; 32];

impl Algorithm<CryptoSHA256Hash> for CryptoYeeAlgorithm {
    #[inline]
    fn hash(&mut self) -> CryptoSHA256Hash {
        let mut h = [0u8; 32];
        self.0.result(&mut h);

        // double sha256
        let mut c = Sha256::new();
        c.input(h.as_ref());
        c.result(&mut h);
        h
    }

    #[inline]
    fn reset(&mut self) {
        self.0.reset();
    }

    fn leaf(&mut self, leaf: CryptoSHA256Hash) -> CryptoSHA256Hash {
        leaf
    }

    fn node(&mut self, left: CryptoSHA256Hash, right: CryptoSHA256Hash) -> CryptoSHA256Hash {
        self.write(left.as_ref());
        self.write(right.as_ref());
        self.hash()
    }

}

pub struct HexSlice<'a>(&'a [u8]);

impl<'a> HexSlice<'a> {
   pub fn new<T>(data: &'a T) -> HexSlice<'a>
        where
            T: ?Sized + AsRef<[u8]> + 'a,
    {
        HexSlice(data.as_ref())
    }
}

/// reverse order
impl<'a> fmt::Display for HexSlice<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let len = self.0.len();
        for i in 0..len {
            let byte = self.0[len - 1 - i];
            write!(f, "{:x}{:x}", byte >> 4, byte & 0xf)?;
        }
        Ok(())
    }
}


