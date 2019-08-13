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

mod number;

use jsonrpc_derive::rpc;
use primitives::{Bytes, sr25519::{Public}};
use crate::Config;
use crate::client::RpcClient;
use crate::errors;
use serde::{Serialize};
use serde::de::DeserializeOwned;
use parity_codec::{Encode, Decode};
use runtime_primitives::OpaqueExtrinsic;
use number::NumberOrHex;
use serde_json::Value;
use crate::rpc::{self, futures::future::{self, FutureResult}};
use futures::Future;
use jsonrpc_client_transports::RpcError;
use jsonrpc_core::BoxFuture;

/// Substrate authoring RPC API
#[rpc]
pub trait ChainApi<Number, Hash> {

	/// Get header of a relay chain block.
	#[rpc(name = "chain_getHeader")]
	fn header(&self, shard_num: u16, hash: Option<Hash>) -> BoxFuture<Option<Value>>;

	/// Get header and body of a relay chain block.
	#[rpc(name = "chain_getBlock")]
	fn block(&self, shard_num: u16, hash: Option<Hash>) -> BoxFuture<Option<Value>>;

	/// Get hash of the n-th block in the canon chain.
	///
	/// By default returns latest block hash.
	#[rpc(name = "chain_getBlockHash", alias("chain_getHead"))]
	fn block_hash(&self, shard_num: u16, number: Option<NumberOrHex<Number>>) -> BoxFuture<Option<Hash>>;
}

/// Chain API
pub struct Chain {
	config : Config,
	rpc_client: RpcClient,
}

impl Chain {
	/// Create new State API RPC handler.
	pub fn new(config: Config) -> Self {
		Self {
			config: config.clone(),
			rpc_client: RpcClient::new(config)
		}
	}
}

impl<Number, Hash> ChainApi<Number, Hash> for Chain
	where Hash: Send + Sync + 'static + Serialize + DeserializeOwned,
		  Number: Send + Sync + 'static + Serialize + DeserializeOwned,
{
	fn header(&self, shard_num: u16, hash: Option<Hash>) -> BoxFuture<Option<Value>>{

		let shard_count = self.config.get_shard_count();

		if shard_num >= shard_count{
			return Box::new(future::err(errors::Error::from(errors::ErrorKind::InvalidShard).into()));
		}

		self.rpc_client.call_method_async("chain_getHeader", "Option<Value>", (hash, ), shard_num)
			.unwrap_or_else(|e|Box::new(future::err(e.into())))

	}

	fn block(&self, shard_num: u16, hash: Option<Hash>) -> BoxFuture<Option<Value>>{

		let shard_count = self.config.get_shard_count();

		if shard_num >= shard_count{
			return Box::new(future::err(errors::Error::from(errors::ErrorKind::InvalidShard).into()));
		}

		self.rpc_client.call_method_async("chain_getBlock", "Option<Value>", (hash, ), shard_num)
			.unwrap_or_else(|e|Box::new(future::err(e.into())))
	}

	fn block_hash(&self, shard_num: u16, number: Option<NumberOrHex<Number>>) -> BoxFuture<Option<Hash>>{

		let shard_count = self.config.get_shard_count();

		if shard_num >= shard_count{
			return Box::new(future::err(errors::Error::from(errors::ErrorKind::InvalidShard).into()));
		}

		self.rpc_client.call_method_async("chain_getBlockHash", "Option<Hash>", (number, ), shard_num)
			.unwrap_or_else(|e|Box::new(future::err(e.into())))
	}

}
