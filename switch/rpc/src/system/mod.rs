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

use jsonrpc_derive::rpc;
use crate::Config;
use crate::client::{RpcClient, Hex};

/// Substrate state API
#[rpc]
pub trait SystemApi {
	/// Returns a storage entry at a specific block's state.
	#[rpc(name = "system_getShardCount")]
	fn shard_count(&self) -> jsonrpc_core::Result<Hex>;
}

/// State API with subscriptions support.
pub struct System {
	config: Config,
	rpc_client: RpcClient,
}

impl System {
	/// Create new State API RPC handler.
	pub fn new(config: Config) -> Self {
		Self {
			config: config.clone(),
			rpc_client: RpcClient::new(config),
        }
	}
}

impl SystemApi for System
{
	fn shard_count(&self) -> jsonrpc_core::Result<Hex> {

		let s = self.config.shards.len() as u32;
		Ok(s.into())
	}
}