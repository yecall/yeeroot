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
use crate::client::{RpcClient, Hex};
use serde::Serialize;
use serde::de::DeserializeOwned;
use parity_codec::{KeyedVec};
use sr_io::blake2_256;
use num_bigint::BigUint;
use yee_runtime::AccountId;
use yee_sharding::utils::shard_num_for_account_id;
use crate::errors;

/// Substrate state API
#[rpc]
pub trait StateApi<Hash> {
	#[rpc(name = "state_getBalance")]
	fn balance(&self, account_id: AccountId) -> jsonrpc_core::Result<Hex>;

	#[rpc(name = "state_getNonce")]
	fn nonce(&self, account_id: AccountId) -> jsonrpc_core::Result<Hex>;
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

	fn get_shard_count(&self)->u16{
		self.config.shards.len() as u16
	}
}

impl<Hash> StateApi<Hash> for State
	where Hash: Send + Sync + 'static + Serialize + DeserializeOwned
{
	fn balance(&self, account_id: AccountId) -> jsonrpc_core::Result<Hex> {

		let shard_count = self.get_shard_count();

		let shard_num = shard_num_for_account_id(&account_id, shard_count).ok_or::<errors::Error>(errors::ErrorKind::InvalidShard.into())?;
		log::debug!("shard_count: {}, shard_num: {}", shard_count, shard_num);

		//free balance
		let key = get_storage_key(&account_id, StorageKeyId::FreeBalance);
		log::debug!("free balance key: {}", hex::encode(key.clone().0));

		let storage : Option<StorageData> = self.rpc_client.call_method("state_getStorage", "Option<StorageData>", (key, Option::<Hash>::None), shard_num)?;
		log::debug!("free balance storage: {}", storage.clone().map(|x: StorageData|hex::encode(x.0)).unwrap_or("".to_string()));

		let free_balance = storage.map(|x|{
			BigUint::from_bytes_le(&x.0)
		}).unwrap_or(BigUint::from(0u64));

		//reserved balance
		let key = get_storage_key(&account_id, StorageKeyId::ReservedBalance);
		log::debug!("reserved balance key: {}", hex::encode(key.clone().0));

		let storage : Option<StorageData> = self.rpc_client.call_method("state_getStorage", "Option<StorageData>", (key, Option::<Hash>::None), shard_num)?;
		log::debug!("free reserved storage: {}", storage.clone().map(|x: StorageData|hex::encode(x.0)).unwrap_or("".to_string()));

		let reserved_balance = storage.map(|x|{
			BigUint::from_bytes_le(&x.0)
		}).unwrap_or(BigUint::from(0u64));

		//sum
		let balance = free_balance + &reserved_balance;

		Ok(balance.into())
	}

	fn nonce(&self, account_id: AccountId) -> jsonrpc_core::Result<Hex> {

		let shard_count = self.get_shard_count();

		let shard_num = shard_num_for_account_id(&account_id, shard_count).ok_or::<errors::Error>(errors::ErrorKind::InvalidShard.into())?;
		log::debug!("shard_count: {}, shard_num: {}", shard_count, shard_num);

		let key = get_storage_key(&account_id, StorageKeyId::AccountNonce);
		log::debug!("nonce key: {}", hex::encode(key.clone().0));

		let storage : Option<StorageData> = self.rpc_client.call_method("state_getStorage", "Option<StorageData>", (key, Option::<Hash>::None), shard_num)?;
		log::debug!("nonce storage: {}", storage.clone().map(|x: StorageData|hex::encode(x.0)).unwrap_or("".to_string()));

		let nonce = storage.map(|x|{
			BigUint::from_bytes_le(&x.0)
		}).unwrap_or(BigUint::from(0u64));

		Ok(nonce.into())
	}
}

enum StorageKeyId{
	FreeBalance,
	ReservedBalance,
	AccountNonce,

}

fn get_prefix(storage_key_id: StorageKeyId) -> &'static [u8]{
	match storage_key_id{
		StorageKeyId::FreeBalance => b"Balances FreeBalance",
		StorageKeyId::ReservedBalance => b"Balances ReservedBalance",
		StorageKeyId::AccountNonce => b"System AccountNonce",
	}
}

fn get_storage_key(account_id: &AccountId, storage_key_id: StorageKeyId) -> StorageKey {
	let a = blake2_256(&account_id.to_keyed_vec(get_prefix(storage_key_id))).to_vec();
	StorageKey(a)
}
