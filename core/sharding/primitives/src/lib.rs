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

///! Primitives for Yee Sharding

pub mod utils;

use {
    substrate_client::decl_runtime_apis,
    runtime_primitives::traits::{NumberFor, Block},
};

#[derive(Clone, Debug)]
pub struct ScaleOut {
    pub shard_num: u16,
}

pub trait ShardingInfo<N> {
    /// get total shard number in genesis block
    fn get_genesis_shard_count() -> N;
    /// get shard number for current chain
    fn get_curr_shard() -> Option<N>;
    /// get total shard number
    fn get_shard_count() -> N;
}

decl_runtime_apis! {
    pub trait ShardingAPI {
        /// get total shard number in genesis block
        fn get_genesis_shard_count() -> u16;
        /// get shard number for current chain
        fn get_curr_shard() -> Option<u16>;
        /// get total shard number
        fn get_shard_count() -> u16;
        /// get scale_out_observe_blocks
        fn get_scale_out_observe_blocks() -> NumberFor<Block>;
    }
}
