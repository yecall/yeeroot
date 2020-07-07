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

#![cfg_attr(not(feature = "std"), no_std)]

///! Primitives for Yee POW

use {
    runtime_primitives::{
        ConsensusEngineId,
        traits::NumberFor,
    },
    substrate_client::decl_runtime_apis,
};
use parity_codec::{Encode, Decode};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

mod big_array;
use big_array::BigArray;

/// `ConsensusEngineId` of Yee POW consensus.
pub const YEE_POW_ENGINE_ID: ConsensusEngineId = [b'Y', b'e', b'e', b'!'];

pub type PowTarget = primitives::U256;

#[derive(Clone, Encode, Decode, Serialize, Deserialize)]
pub struct ExtraData(#[serde(with = "BigArray")][u8; 40]);

impl ExtraData {
    pub fn from(input: [u8; 40]) -> Self{
        ExtraData(input)
    }
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl Default for ExtraData {
    fn default() -> Self {
        ExtraData([0u8;40])
    }
}

impl std::fmt::Debug for ExtraData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for ExtraData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let v = self.0.to_vec();
        let s = format!("{:?}", v);
        f.write_str(s.as_str())
    }
}

impl PartialEq for ExtraData {
    fn eq(&self, other: &ExtraData) -> bool {
        if self.0.to_vec().as_slice().cmp(other.0.to_vec().as_slice()) == std::cmp::Ordering::Equal {
            return true
        }
        return false
    }
}

decl_runtime_apis! {
    pub trait YeePOWApi {
        /// POW target config used for genesis block
        fn genesis_pow_target() -> PowTarget;

        /// In-Chain config for POW target adjust period
        fn pow_target_adj() -> NumberFor<Block>;

        /// Target block time in seconds
        fn target_block_time() -> u64;
    }
}
