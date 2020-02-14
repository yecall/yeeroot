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
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use parity_codec::{KeyedVec, Codec, Decode, Encode, Input, Compact};
use sr_io::blake2_256;
use num_bigint::BigUint;
use yee_runtime::AccountId;
use yee_sharding_primitives::utils::shard_num_for_bytes;
use crate::errors;
use jsonrpc_core::{BoxFuture, Error, ErrorCode};
use crate::rpc::{self, futures::future::{self, FutureResult}};
use yee_serde_hex::Hex;
use yee_primitives::{Address, AddressCodec, Hrp};
use serde_json::Value;
use hex;
use yee_sr_primitives::SHARD_CODE_SIZE;
use serde_json::map::Entry::Vacant;

/// Substrate state API
#[rpc]
pub trait StateApi<Hash> {
	#[rpc(name = "state_getBalance")]
	fn balance(&self, address: Address, hash: Option<Hash>) -> BoxFuture<Hex<BigUint>>;

	#[rpc(name = "state_getNonce")]
	fn nonce(&self, account: Address, hash: Option<Hash>) -> BoxFuture<Hex<BigUint>>;

	#[rpc(name = "state_getAssetBalance")]
	fn asset_balance(&self, address: Address, shard_code: String, asset_id: u32, hash: Option<Hash>) -> BoxFuture<Hex<BigUint>>;

	#[rpc(name = "state_getAssetDetail")]
	fn asset_detail(&self, shard_num: u16, asset_id: u32, hash: Option<Hash>) -> BoxFuture<Option<AssetDetail>>;

	/// Get block events.
	#[rpc(name = "state_getBlockEvents")]
	fn block_events(&self, shard_num: u16, hash: Option<Hash>) -> BoxFuture<Vec<u8>>;
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct AssetDetail {
	pub shard_code: Vec<u8>,
	pub id: u32,
	pub name: Vec<u8>,
	pub decimals: u16,
	pub total_supply: Hex<BigUint>,
	pub issuer: Vec<u8>
}

/// State API with subscriptions support.
pub struct State {
	config: Config,
	rpc_client: RpcClient,
}

const BLOCK_EVENTS_PREFIX: &'static str = "0xcc956bdb7605e3547539f321ac2bc95c";

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

	fn asset_balance(&self, address: Address, shard_code: String, asset_id: u32, hash: Option<Hash>) -> BoxFuture<Hex<BigUint>> {
		let account_id = match AccountId::from_address(&address) {
			Ok((id, _)) => id,
			Err(_e) => return Box::new(future::err(errors::Error::from(errors::ErrorKind::InvalidAddress).into()))
		};
		let shard_count = self.config.get_shard_count();
		let shard_num = match shard_num_for_bytes(account_id.as_slice(), shard_count) {
			Some(num) => num,
			None => return Box::new(future::err(errors::Error::from(errors::ErrorKind::InvalidShard).into())),
		};
		let shard_code = match hex::decode(shard_code){
			Ok(code) => code,
			Err(e) => return Box::new(future::err(errors::Error::from(errors::ErrorKind::InvalidShardCode).into()))
		};
		// key
		let k = (shard_code, asset_id, account_id);
		let key = get_storage_key(&k, StorageKeyId::AssetBalance);
		let balance_future = match self.rpc_client.call_method_async("state_getStorage", "Option<StorageData>", (key, hash.clone()), shard_num) {
			Ok(future) => future.map(|b| {
				Hex(get_big_uint(b))
			}),
			Err(e) => return Box::new(future::err(e.into()))
		};
		Box::new(balance_future)
	}

	fn asset_detail(&self, shard_num: u16, asset_id: u32, hash: Option<Hash>) -> BoxFuture<Option<AssetDetail>> {
		let key = get_storage_key(&asset_id, StorageKeyId::AssetName);
		let name_future = match self.rpc_client.call_method_async("state_getStorage", "Option<StorageData>", (key,hash.clone()), shard_num) {
			Ok(future) => future.map(|b: Option<StorageData>| {
				 match b {
					Some(d) => d.0,
					None => vec![]
				}
			}),
			Err(e) => return  Box::new(future::err(e.into())),
		};

		let key = get_storage_key(&asset_id, StorageKeyId::AssetTotalSupply);
		let supply_future = match self.rpc_client.call_method_async("state_getStorage", "Option<StorageData>", (key,hash.clone()), shard_num) {
			Ok(future) => future.map(|b: Option<StorageData>| {
				Hex(get_big_uint(b))
			}),
			Err(e) => return  Box::new(future::err(e.into())),
		};

		let key = get_storage_key(&asset_id, StorageKeyId::AssetDecimals);
		let decimals_future = match self.rpc_client.call_method_async("state_getStorage", "Option<StorageData>", (key,hash.clone()), shard_num) {
			Ok(future) => future.map(|b: Option<StorageData>| {
				match b {
					Some(d) => {
						let d = d.0.as_slice();
						let mut arr = [0u8; 2];
						arr.copy_from_slice(d);
						u16::from_le_bytes(arr)
					},
					None => 0
				}
			}),
			Err(e) => return  Box::new(future::err(e.into())),
		};

		let key = get_storage_key(&asset_id, StorageKeyId::AssetIssuer);
		let issuer_future = match self.rpc_client.call_method_async("state_getStorage", "Option<StorageData>", (key,hash.clone()), shard_num) {
			Ok(future) => future.map(|b: Option<StorageData>| {
				match b {
					Some(d) => d.0,
					None => vec![]
				}
			}),
			Err(e) => return  Box::new(future::err(e.into())),
		};

		Box::new(name_future.join4(supply_future, decimals_future, issuer_future).map(move |(name, supply, decimals, issuer)| {
			let shard_code = if issuer.len() > 2 {
				issuer[issuer.len() - SHARD_CODE_SIZE..].to_vec()
			} else {
				vec![]
			};
			Some(AssetDetail {
				shard_code,
				id: asset_id,
				name: match Decode::decode(&mut name.as_slice()) {
					Some(name) => name,
					None => vec![]
				},
				total_supply: supply,
				decimals,
				issuer,
			})
		}))
	}

	fn block_events(&self, shard_num: u16, hash: Option<Hash>) -> BoxFuture<Vec<u8>> {
		let shard_count = self.config.get_shard_count();
		if shard_num >= shard_count {
			return Box::new(future::err(errors::Error::from(errors::ErrorKind::InvalidShard).into()));
		}

		match self.rpc_client.call_method_async("state_getStorage", "Option<StorageData>", (BLOCK_EVENTS_PREFIX, hash), shard_num) {
			Ok(future) => {
				Box::new(future.map(|result: Option<StorageData>| {
					log::debug!("block_events storage: {}", result.clone().map(|x: StorageData|hex::encode(x.0)).unwrap_or("".to_string()));
					match result {
						Some(d) => d.0,
						None => vec![]
					}
				}))
			},
			Err(e) => {
				Box::new(future::err(e.into()))
			}
		}
	}
}

enum StorageKeyId {
	FreeBalance,
	ReservedBalance,
	AccountNonce,
	AssetBalance,
	AssetName,
	AssetIssuer,
	AssetDecimals,
	AssetTotalSupply,
	AssetNextId,
}

fn get_prefix(storage_key_id: StorageKeyId) -> &'static [u8] {
	match storage_key_id {
		StorageKeyId::FreeBalance => b"Balances FreeBalance",
		StorageKeyId::ReservedBalance => b"Balances ReservedBalance",
		StorageKeyId::AccountNonce => b"System AccountNonce",
		StorageKeyId::AssetBalance => b"Assets Balances",
		StorageKeyId::AssetName => b"Assets AssetsName",
		StorageKeyId::AssetIssuer => b"Assets AssetsIssuer",
		StorageKeyId::AssetDecimals => b"Assets AssetsDecimals",
		StorageKeyId::AssetTotalSupply => b"Assets TotalSupply",
		StorageKeyId::AssetNextId => b"Assets NextAssetId",
	}
}

fn get_storage_key<T>(key: &T, storage_key_id: StorageKeyId) -> StorageKey where T: Codec {
	let a = blake2_256(&key.to_keyed_vec(get_prefix(storage_key_id))).to_vec();
	StorageKey(a)
}

fn get_big_uint(result: Option<StorageData>) -> BigUint {
	result.map(|x| {
		BigUint::from_bytes_le(&x.0)
	}).unwrap_or(BigUint::from(0u64))
}
