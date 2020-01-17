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
use primitives::{sr25519, storage::{StorageKey, StorageData}};
use crate::rpc::futures::{Future, Stream};
use crate::Config;
use crate::client::RpcClient;
use serde::Serialize;
use serde::de::DeserializeOwned;
use parity_codec::{KeyedVec, Encode};
use sr_io::blake2_256;
use num_bigint::BigUint;
use yee_runtime::AccountId;
use yee_sharding_primitives::utils::shard_num_for_bytes;
use crate::errors;
use jsonrpc_core::BoxFuture;
use crate::rpc::{self, futures::future::{self, FutureResult}};
use yee_serde_hex::Hex;
use yee_primitives::{Address, AddressCodec};

/// Substrate state API
#[rpc]
pub trait StateApi<Hash> {
	#[rpc(name = "state_getBalance")]
	fn balance(&self, address: Address, hash: Option<Hash>) -> BoxFuture<Hex<BigUint>>;

	#[rpc(name = "state_getNonce")]
	fn nonce(&self, account: Address, hash: Option<Hash>) -> BoxFuture<Hex<BigUint>>;

	#[rpc(name = "state_getAssetBalance")]
	fn asset_balance(&self, address: Address, asset_id: u32, hash: Option<Hash>) -> BoxFuture<Hex<BigUint>>;
}

/// State API with subscriptions support.
pub struct State {
	config: Config,
	rpc_client: RpcClient,
}

impl State {
	/// Create new State API RPC handler.
	pub fn new(config: Config) -> Self {
		Self {
			config: config.clone(),
			rpc_client: RpcClient::new(config),
		}
	}
}

impl<Hash> StateApi<Hash> for State
	where Hash: Send + Sync + 'static + Serialize + DeserializeOwned + Clone
{
	fn balance(&self, address: Address, hash: Option<Hash>) -> BoxFuture<Hex<BigUint>> {
		let account_id = match AccountId::from_address(&address) {
			Ok((account_id, _hrp)) => account_id,
			Err(e) => return Box::new(future::err(errors::Error::from(errors::ErrorKind::InvalidAddress).into())),
		};

		let shard_count = self.config.get_shard_count();

		let shard_num = match shard_num_for_bytes(account_id.as_slice(), shard_count) {
			Some(shard_num) => shard_num,
			None => return Box::new(future::err(errors::Error::from(errors::ErrorKind::InvalidShard).into())),
		};
		log::debug!("shard_count: {}, shard_num: {}", shard_count, shard_num);

		//free balance
		let free_balance_key = get_storage_key(&account_id, StorageKeyId::FreeBalance);
		log::debug!("free balance key: {}", hex::encode(free_balance_key.clone().0));

		//reserved balance
		let reserved_balance_key = get_storage_key(&account_id, StorageKeyId::ReservedBalance);
		log::debug!("reserved balance key: {}", hex::encode(reserved_balance_key.clone().0));

		let free_balance_future: BoxFuture<Option<StorageData>> = match self.rpc_client.call_method_async("state_getStorage", "Option<StorageData>", (free_balance_key, hash.clone()), shard_num) {
			Ok(future) => future,
			Err(e) => return Box::new(future::err(e.into())),
		};

		let reserved_balance_future: BoxFuture<Option<StorageData>> = match self.rpc_client.call_method_async("state_getStorage", "Option<StorageData>", (reserved_balance_key, hash), shard_num) {
			Ok(future) => future,
			Err(e) => return Box::new(future::err(e.into())),
		};


		Box::new(free_balance_future.join(reserved_balance_future).map(|(result1, result2)| {
			log::debug!("free balance storage: {}", result1.clone().map(|x: StorageData|hex::encode(x.0)).unwrap_or("".to_string()));

			log::debug!("reserved balance storage: {}", result2.clone().map(|x: StorageData|hex::encode(x.0)).unwrap_or("".to_string()));

			let free_balance = get_big_uint(result1);

			let reserved_balance = get_big_uint(result2);

			Hex(free_balance + reserved_balance)
		}))
	}

	fn nonce(&self, address: Address, hash: Option<Hash>) -> BoxFuture<Hex<BigUint>> {
		let account_id = match AccountId::from_address(&address) {
			Ok((account_id, _hrp)) => account_id,
			Err(e) => return Box::new(future::err(errors::Error::from(errors::ErrorKind::InvalidAddress).into())),
		};

		let shard_count = self.config.get_shard_count();

		let shard_num = match shard_num_for_bytes(account_id.as_slice(), shard_count) {
			Some(shard_num) => shard_num,
			None => return Box::new(future::err(errors::Error::from(errors::ErrorKind::InvalidShard).into())),
		};
		log::debug!("shard_count: {}, shard_num: {}", shard_count, shard_num);

		let key = get_storage_key(&account_id, StorageKeyId::AccountNonce);
		log::debug!("nonce key: {}", hex::encode(key.clone().0));

		match self.rpc_client.call_method_async("state_getStorage", "Option<StorageData>", (key, hash), shard_num) {
			Ok(future) => {
				Box::new(future.map(|result: Option<StorageData>| {
					log::debug!("nonce storage: {}", result.clone().map(|x: StorageData|hex::encode(x.0)).unwrap_or("".to_string()));

					Hex(get_big_uint(result))
				}))
			}
			Err(e) => {
				Box::new(future::err(e.into()))
			}
		}
	}

	fn asset_balance(&self, address: Address, asset_id: u32, hash: Option<Hash>) -> BoxFuture<Hex<BigUint>> {
		let account_id = match AccountId::from_address(&address) {
			Ok((id, _)) => id,
			Err(_e) => return Box::new(future::err(errors::Error::from(errors::ErrorKind::InvalidAddress).into()))
		};
		let shard_count = self.config.get_shard_count();
		let shard_num = match shard_num_for_bytes(account_id.as_slice(), shard_count) {
			Some(num) => num,
			None => return Box::new(future::err(errors::Error::from(errors::ErrorKind::InvalidShard).into())),
		};
		// key
		let key = get_asset_storage_key(&account_id, asset_id);
		let balance_future = match self.rpc_client.call_method_async("state_getStorage", "Option<StorageData>", (key, hash.clone()), shard_num) {
			Ok(future) => future.map(|b| {
				Hex(get_big_uint(b))
			}),
			Err(e) => return Box::new(future::err(e.into()))
		};
		Box::new(balance_future)
	}
}

enum StorageKeyId {
	FreeBalance,
	ReservedBalance,
	AccountNonce,
	AssetBalance,
}

fn get_prefix(storage_key_id: StorageKeyId) -> &'static [u8] {
	match storage_key_id {
		StorageKeyId::FreeBalance => b"Balances FreeBalance",
		StorageKeyId::ReservedBalance => b"Balances ReservedBalance",
		StorageKeyId::AccountNonce => b"System AccountNonce",
		StorageKeyId::AssetBalance => b"Assets Balances",
	}
}

fn get_storage_key(account_id: &AccountId, storage_key_id: StorageKeyId) -> StorageKey {
	let a = blake2_256(&account_id.to_keyed_vec(get_prefix(storage_key_id))).to_vec();
	StorageKey(a)
}

fn get_asset_storage_key(account_id: &AccountId, asset_id: u32) -> StorageKey {
	let mut key = get_prefix(StorageKeyId::AssetBalance).to_vec();
	let x = (account_id, asset_id);
	Encode::encode_to(&x, &mut key);
	StorageKey(blake2_256(key.as_slice()).to_vec())
	//StorageKey(key)
}

fn get_big_uint(result: Option<StorageData>) -> BigUint {
	result.map(|x| {
		BigUint::from_bytes_le(&x.0)
	}).unwrap_or(BigUint::from(0u64))
}
