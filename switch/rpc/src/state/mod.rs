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
use primitives::storage::{StorageKey, StorageData};
use crate::rpc::futures::{Future, Sink, Stream};
use crate::Config;
use crate::client::RpcClient;
use runtime_version::RuntimeVersion;
use jsonrpc_core_client::TypedClient;
use std::time::Duration;
use serde::{Serialize};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Substrate state API
#[rpc]
pub trait StateApi<Hash> {

	/// Returns a storage entry at a specific block's state.
	#[rpc(name = "state_getStorage", alias("state_getStorageAt"))]
	fn storage(&self, key: StorageKey, hash: Option<Hash>) -> jsonrpc_core::Result<Option<StorageData>>;

	/// Returns the runtime metadata as an opaque blob.
	#[rpc(name = "state_getMetadata")]
	fn metadata(&self, hash: Option<Hash>) -> jsonrpc_core::Result<Bytes>;

	/// Get the runtime version.
	#[rpc(name = "state_getRuntimeVersion", alias("chain_getRuntimeVersion"))]
	fn runtime_version(&self, hash: Option<Hash>) -> jsonrpc_core::Result<Value>;

}

/// State API with subscriptions support.
pub struct State {
	config : Config,
	rpc_client: RpcClient,
}

impl State {
	/// Create new State API RPC handler.
	pub fn new(config: Config) -> Self {
		Self {
			config: config.clone(),
			rpc_client: RpcClient::new(config)
		}
	}
}

impl<Hash> StateApi<Hash> for State
	where Hash: Send + Sync + 'static + Serialize + DeserializeOwned
{
	fn storage(&self, key: StorageKey, block: Option<Hash>) -> jsonrpc_core::Result<Option<StorageData>> {

		self.rpc_client.call_method("state_getStorage", "Option<StorageData>", (key, block))

	}

	fn metadata(&self, hash: Option<Hash>) -> jsonrpc_core::Result<Bytes> {

		self.rpc_client.call_method("state_getMetadata", "Bytes", (hash,))

	}

	fn runtime_version(&self, hash: Option<Hash>) -> jsonrpc_core::Result<Value> {

		self.rpc_client.call_method("state_getRuntimeVersion", "Value", (hash,))

	}
}
