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
use primitives::{Bytes, sr25519::{Public}};
use crate::Config;
use crate::client::RpcClient;
use crate::errors;
use serde::{Serialize};
use serde::de::DeserializeOwned;
use parity_codec::{Encode, Decode};
use runtime_primitives::OpaqueExtrinsic;
use yee_sharding_primitives::utils::shard_num_for_bytes;
use jsonrpc_core::BoxFuture;
use crate::rpc::{self, futures::future::{self, FutureResult}};

/// Substrate authoring RPC API
#[rpc]
pub trait AuthorApi<Hash> {

	/// Submit hex-encoded extrinsic for inclusion in block.
	#[rpc(name = "author_submitExtrinsic")]
	fn submit_extrinsic(&self, extrinsic: Bytes) -> BoxFuture<Hash>;
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
	fn submit_extrinsic(&self, extrinsic: Bytes) -> BoxFuture<Hash> {

		let xt : OpaqueExtrinsic = match Decode::decode(&mut &extrinsic[..]){
			Some(xt) => xt,
			None => return Box::new(future::err(errors::Error::from(errors::ErrorKind::ParseError).into())),
		};

		let bytes : &[u8] = &xt.0;

		let version = bytes[0];

		let is_signed = version & 0b1000_0000 != 0;

		if !is_signed {
			return Box::new(future::err(errors::Error::from(errors::ErrorKind::ParseError).into()));
		}

		let address = &bytes[1..34];//0xFF + 32bytes

		let address = &address[1..];//trim 0xFF

		log::debug!("xt={:?}, version={}, account_id={}", xt, version, Public::from_slice(address));

		let shard_count = self.config.get_shard_count();

		let shard_num = match shard_num_for_bytes(address, shard_count){
			Some(shard_num) => shard_num,
			None => return Box::new(future::err(errors::Error::from(errors::ErrorKind::InvalidShard).into())),
		};

		log::debug!("shard_count: {}, shard_num: {}", shard_count, shard_num);

		self.rpc_client.call_method_async("author_submitExtrinsic", "Hash", (extrinsic,), shard_num)
			.unwrap_or_else(|e|Box::new(future::err(e.into())))
	}

}
