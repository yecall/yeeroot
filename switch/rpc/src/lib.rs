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

pub mod metadata;
pub mod author;
pub mod state;
pub mod system;
pub mod chain;
pub mod pow;
mod errors;
mod client;
use jsonrpc_core as rpc;
use parity_codec::alloc::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Config{
    pub shards: HashMap<String, Shard>,
}

#[derive(Clone, Debug)]
pub struct Shard {
    pub rpc: Vec<String>,
}

impl Config{
    fn get_shard_count(&self)->u16{
        self.shards.len() as u16
    }
}
