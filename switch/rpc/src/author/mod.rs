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

use log::{info};
use jsonrpc_derive::rpc;
use primitives::{Bytes};
use crate::Config;
use crate::client::RpcClient;
use serde::{Serialize};
use serde::de::DeserializeOwned;

/// Substrate authoring RPC API
#[rpc]
pub trait AuthorApi<Hash> {

	/// Submit hex-encoded extrinsic for inclusion in block.
	#[rpc(name = "author_submitExtrinsic")]
	fn submit_extrinsic(&self, extrinsic: Bytes) -> jsonrpc_core::Result<Hash>;
}

/// Authoring API
pub struct Author {
	config : Config,
	rpc_client: RpcClient,
}

impl Author {
	/// Create new State API RPC handler.
	pub fn new(config: Config) -> Self {
		Self {
			config: config.clone(),
			rpc_client: RpcClient::new(config)
		}
	}
}

impl<Hash> AuthorApi<Hash> for Author
	where Hash: Send + Sync + 'static + Serialize + DeserializeOwned
{
	fn submit_extrinsic(&self, extrinsic: Bytes) -> jsonrpc_core::Result<Hash> {

		self.rpc_client.call_method("author_submitExtrinsic", "Hash", (extrinsic, ))
	}

}
