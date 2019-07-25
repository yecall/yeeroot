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

use std::{
	collections::{BTreeMap, HashMap},
	ops::Range,
	sync::Arc,
};

use error_chain::bail;
use log::{warn, trace, info};
use client::{self, Client, CallExecutor, BlockchainEvents, runtime_api::Metadata};
use jsonrpc_derive::rpc;
use jsonrpc_pubsub::{typed::Subscriber, SubscriptionId};
use primitives::{H256, Blake2Hasher, Bytes};
use primitives::hexdisplay::HexDisplay;
use primitives::storage::{self, StorageKey, StorageData, StorageChangeSet};
use crate::rpc::Result as RpcResult;
use crate::rpc::futures::{stream, Future, Sink, Stream};
use runtime_primitives::generic::BlockId;
use runtime_primitives::traits::{Block as BlockT, Header, ProvideRuntimeApi, As, NumberFor};
use runtime_version::RuntimeVersion;
use state_machine::{self, ExecutionStrategy};

mod error;

use self::error::Result;

/// Substrate state API
#[rpc]
pub trait StateApi<Hash> {

	/// Call a contract at a block's state.
	#[rpc(name = "state_call", alias("state_callAt"))]
	fn call(&self, name: String, bytes: Bytes, hash: Option<Hash>) -> Result<Bytes>;

	/// Returns the keys with prefix, leave empty to get all the keys
	#[rpc(name = "state_getKeys")]
	fn storage_keys(&self, key: StorageKey, hash: Option<Hash>) -> Result<Vec<StorageKey>>;

	/// Returns a storage entry at a specific block's state.
	#[rpc(name = "state_getStorage", alias("state_getStorageAt"))]
	fn storage(&self, key: StorageKey, hash: Option<Hash>) -> Result<Option<StorageData>>;

	/// Returns the hash of a storage entry at a block's state.
	#[rpc(name = "state_getStorageHash", alias("state_getStorageHashAt"))]
	fn storage_hash(&self, key: StorageKey, hash: Option<Hash>) -> Result<Option<Hash>>;

	/// Returns the size of a storage entry at a block's state.
	#[rpc(name = "state_getStorageSize", alias("state_getStorageSizeAt"))]
	fn storage_size(&self, key: StorageKey, hash: Option<Hash>) -> Result<Option<u64>>;

	/// Returns the runtime metadata as an opaque blob.
	#[rpc(name = "state_getMetadata")]
	fn metadata(&self, hash: Option<Hash>) -> Result<Bytes>;

	/// Get the runtime version.
	#[rpc(name = "state_getRuntimeVersion", alias("chain_getRuntimeVersion"))]
	fn runtime_version(&self, hash: Option<Hash>) -> Result<RuntimeVersion>;

	/// Query historical storage entries (by key) starting from a block given as the second parameter.
	///
	/// NOTE This first returned result contains the initial state of storage for all keys.
	/// Subsequent values in the vector represent changes to the previous state (diffs).
	#[rpc(name = "state_queryStorage")]
	fn query_storage(&self, keys: Vec<StorageKey>, block: Hash, hash: Option<Hash>) -> Result<Vec<StorageChangeSet<Hash>>>;
}

/// State API with subscriptions support.
pub struct State {
}

impl State {
	/// Create new State API RPC handler.
	pub fn new() -> Self {
		Self {
		}
	}
}

impl<Hash> StateApi<Hash> for State where
{
	fn call(&self, method: String, data: Bytes, block: Option<Hash>) -> Result<Bytes> {
		info!("call");

		unimplemented!("");
	}

	fn storage_keys(&self, key_prefix: StorageKey, block: Option<Hash>) -> Result<Vec<StorageKey>> {
		info!("storage_keys");

		unimplemented!("");
	}

	fn storage(&self, key: StorageKey, block: Option<Hash>) -> Result<Option<StorageData>> {
		info!("storage");

		unimplemented!("");
	}

	fn storage_hash(&self, key: StorageKey, block: Option<Hash>) -> Result<Option<Hash>> {
		info!("storage_hash");

		unimplemented!("");
	}

	fn storage_size(&self, key: StorageKey, block: Option<Hash>) -> Result<Option<u64>> {
		info!("storage_size");

		unimplemented!("");
	}

	fn metadata(&self, block: Option<Hash>) -> Result<Bytes> {
		info!("metadata");

		unimplemented!("");
	}

	fn runtime_version(&self, at: Option<Hash>) -> Result<RuntimeVersion> {
		info!("runtime_version");

		unimplemented!("");
	}

	fn query_storage(
		&self,
		keys: Vec<StorageKey>,
		from: Hash,
		to: Option<Hash>
	) -> Result<Vec<StorageChangeSet<Hash>>> {
		info!("query_storage");

		unimplemented!("");
	}


}