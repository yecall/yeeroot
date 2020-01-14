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

//! A simple, secure module for dealing with fungible assets.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use srml_support::{StorageValue, StorageMap, Parameter, decl_module, decl_event, decl_storage, ensure, dispatch::Result};
use primitives::{traits::{Member, SimpleArithmetic, As, Zero, StaticLookup}, generic::Era};
use sharding_primitives::ShardingInfo;
use parity_codec::{Decode, Encode, Compact, Input};
use system::ensure_signed;
use rstd::prelude::Vec;

pub trait Trait: sharding::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// The units in which we record balances.
	type Balance: Member + Parameter + SimpleArithmetic + Default + Copy;

	type Sharding: ShardingInfo<Self::ShardNum>;
}

type AssetId = u32;

type AssetName = Vec<u8>;

type Decimals = u16;

const MAX_NAME_SIZE: usize = 16;

decl_module! {
	// Simple declaration of the `Module` type. Lets the macro know what its working on.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;
		/// Issue a new class of fungible assets. There are, and will only ever be, `total`
		/// such assets and they'll all belong to the `origin` initially. It will have an
		/// identifier `AssetId` instance: this will be specified in the `Issued` event.
		fn issue(origin, name: AssetName, #[compact] total: T::Balance, decimals: Decimals) -> Result {
			let origin = ensure_signed(origin)?;

			if name.len() > MAX_NAME_SIZE {
				return Err("Asset's name's length overflow.")
			}

			let id = Self::next_asset_id();
			<NextAssetId<T>>::mutate(|id| *id += 1);

			<Balances<T>>::insert((id, origin.clone()), total);
			<TotalSupply<T>>::insert(id, total);
			<AssetsName<T>>::insert(id, name.clone());
			<AssetsDecimals<T>>::insert(id, decimals);
			<AssetsIssuer<T>>::insert(id, origin.clone());

			// event
			Self::deposit_event(RawEvent::Issued(id, name, origin, total));
			Ok(())
		}

		/// Move some assets from one holder to another.
		fn transfer(origin,
			#[compact] id: AssetId,
			target: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::Balance
		) {
			let origin = ensure_signed(origin)?;
			let origin_account = (id, origin.clone());
			let origin_balance = <Balances<T>>::get(&origin_account);

			ensure!(!amount.is_zero(), "transfer amount should be non-zero");
			ensure!(origin_balance >= amount, "origin account balance must be greater than or equal to the transfer amount");

			// change amount about origin account
			<Balances<T>>::insert(origin_account, origin_balance - amount);
			let target = T::Lookup::lookup(target)?;

			let (cn, c) = (T::Sharding::get_curr_shard().expect("can't get current shard num").as_() as u16, T::Sharding::get_shard_count().as_() as u16);
			let dn = sharding_primitives::utils::shard_num_for(&target, c).expect("can't get target shard num");
			// in same sharding
			if cn == dn {
				<Balances<T>>::mutate((id, target.clone()), |balance| *balance += amount);
			}
			// event
			Self::deposit_event(RawEvent::Transferred(id, origin, target, amount));
		}
	}
}

decl_event!(
	pub enum Event<T> where <T as system::Trait>::AccountId, <T as Trait>::Balance {
		/// Some assets were issued.
		Issued(AssetId, AssetName, AccountId, Balance),
		/// Some assets were transferred.
		Transferred(AssetId, AccountId, AccountId, Balance),
	}
);

decl_storage! {
	trait Store for Module<T: Trait> as Assets {
		/// The number of units of assets held by any given account.
		Balances: map (AssetId, T::AccountId) => T::Balance;
		/// The next asset identifier up for grabs.
		NextAssetId get(next_asset_id): AssetId;
		/// The name of an asset.
		AssetsName: map AssetId => AssetName;
		/// The total unit supply of an asset
		TotalSupply: map AssetId => T::Balance;
		/// The Asset's decimals.
		AssetsDecimals: map AssetId => Decimals;
		/// The asset's issuer.
		AssetsIssuer: map AssetId => T::AccountId;
	}
}

// The main implementation block for the module.
impl<T: Trait> Module<T> {
	/// Get the asset `id` balance of `who`.
	pub fn balance(id: AssetId, who: T::AccountId) -> T::Balance {
		<Balances<T>>::get((id, who))
	}

	/// Get the total supply of an asset `id`
	pub fn total_supply(id: AssetId) -> T::Balance {
		<TotalSupply<T>>::get(id)
	}

	/// Get the name of an asset `id`
	pub fn name(id: AssetId) -> AssetName { <AssetsName<T>>::get(id) }

	/// Get the decimals of an asset `id`
	pub fn decimals(id: AssetId) -> Decimals { <AssetsDecimals<T>>::get(id) }

	/// Get the issuer of an asset `id`
	pub fn issuer(id: AssetId) -> T::AccountId { <AssetsIssuer<T>>::get(id) }

	/// relay transfer
	pub fn relay_transfer(input: Vec<u8>) -> Result {
		if let Some(tx) = Self::decode(input) {
			<Balances<T>>::mutate((tx.id(), tx.to()), |balance| *balance += tx.amount());
			Self::deposit_event(RawEvent::Transferred(tx.id(), tx.from(), tx.to(), tx.amount()));
			Ok(())
		} else{
			Err("transfer is invalid.")
		}
	}

	/// decode from input
	fn decode(input: Vec<u8>) -> Option<OriginAsset<T::AccountId, T::Balance>> {
		// todo

		let mut input = input.as_slice();
		if input.len() < 64 + 1 + 1 {
			return None;
		}
		// length
		let _len: Vec<()> = match Decode::decode(&mut input) {
			Some(len) => len,
			None => return None
		};
		// version
		let version = match input.read_byte() {
			Some(v) => v,
			None => return None
		};
		// is signed
		let is_signed = version & 0b1000_0000 != 0;
		let version = version & 0b0111_1111;
		if version != 1u8 {
			return None;
		}

		let (sender, signature, index, era) = if is_signed {
			// sender type
			let _type = match input.read_byte() {
				Some(a_t) => a_t,
				None => return None
			};
			// sender
			let sender = match Decode::decode(&mut input) {
				Some(s) => s,
				None => return None
			};
			if input.len() < 64 {
				return None;
			}
			// signature
			let signature = input[..64].to_vec();
			input = &input[64..];
			// index
			let index = match Decode::decode(&mut input) {
				Some(i) => i,
				None => return None
			};
			if input.len() < 1 {
				return None;
			}
			// era
			let era = if input[0] != 0u8 {
				match Decode::decode(&mut input) {
					Some(e) => e,
					None => return None
				}
			} else {
				input = &input[1..];
				Era::Immortal
			};
			(sender, signature, index, era)
		} else {
			(T::AccountId::default(), Vec::new(), Compact(0u64), Era::Immortal)
		};

		if input.len() < 2 + 32 + 1 {
			return None;
		}
		// module
		let _module: u8 = match input.read_byte() {
			Some(m) => m,
			None => return None
		};
		// function
		let _func: u8 = match input.read_byte() {
			Some(f) => f,
			None => return None
		};
		// AssetId
		let id: Compact<u32> = match Decode::decode(&mut input) {
			Some(id) => id,
			None => return None
		};
		// dest address type
		let _type: u8 = match input.read_byte() {
			Some(t) => t,
			None => return None
		};
		// dest address
		let dest: T::AccountId = match Decode::decode(&mut input) {
			Some(addr) => addr,
			None => return None
		};
		// amount
		let amount: T::Balance = match Decode::decode(&mut input) {
			Some(a) => {
				let a_c: Compact<u128> = a;
				let buf = a_c.0.encode();
				match Decode::decode(&mut buf.as_slice()) {
					Some(am) => am,
					None => return None
				}
			}
			None => return None
		};
		None
	}
}

/// OriginAsset for asset transfer
struct OriginAsset<Address, Balance> where Address: Clone, Balance: Clone {
	id: u32,
	sender: Address,
	signature: Vec<u8>,
	index: Compact<u64>,
	era: Era,
	dest: Address,
	amount: Balance,
}

impl<Address, Balance> OriginAsset<Address, Balance>  where Address: Clone, Balance: Clone {
	pub fn id(&self) -> u32{
		self.id
	}

	pub fn from(&self) -> Address {
		self.sender.clone()
	}

	pub fn to(&self) -> Address {
		self.dest.clone()
	}

	pub fn signature(&self) -> Vec<u8> {
		self.signature.clone()
	}

	pub fn index(&self) -> Compact<u64> {
		self.index.clone()
	}

	pub fn era(&self) -> Era{
		self.era.clone()
	}

	pub fn amount(&self) -> Balance {
		self.amount.clone()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	use runtime_io::with_externalities;
	use srml_support::{impl_outer_origin, assert_ok, assert_noop};
	use substrate_primitives::{H256, Blake2Hasher};
	// The testing primitives are very useful for avoiding having to work with signatures
	// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
	use primitives::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header}
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type Log = DigestItem;
	}
	impl Trait for Test {
		type Event = ();
		type Balance = u64;
		type Sharding = ();
	}
	type Assets = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
	}

	#[test]
	fn issuing_asset_units_to_issuer_should_work() {
		with_externalities(&mut new_test_ext(), || {
			assert_ok!(Assets::issue(Origin::signed(1), 100));
			assert_eq!(Assets::balance(0, 1), 100);
		});
	}

	#[test]
	fn querying_total_supply_should_work() {
		with_externalities(&mut new_test_ext(), || {
			assert_ok!(Assets::issue(Origin::signed(1), 100));
			assert_eq!(Assets::balance(0, 1), 100);
			assert_ok!(Assets::transfer(Origin::signed(1), 0, 2, 50));
			assert_eq!(Assets::balance(0, 1), 50);
			assert_eq!(Assets::balance(0, 2), 50);
			assert_ok!(Assets::transfer(Origin::signed(2), 0, 3, 31));
			assert_eq!(Assets::balance(0, 1), 50);
			assert_eq!(Assets::balance(0, 2), 19);
			assert_eq!(Assets::balance(0, 3), 31);
			assert_ok!(Assets::destroy(Origin::signed(3), 0));
			assert_eq!(Assets::total_supply(0), 69);
		});
	}

	#[test]
	fn transferring_amount_above_available_balance_should_work() {
		with_externalities(&mut new_test_ext(), || {
			assert_ok!(Assets::issue(Origin::signed(1), 100));
			assert_eq!(Assets::balance(0, 1), 100);
			assert_ok!(Assets::transfer(Origin::signed(1), 0, 2, 50));
			assert_eq!(Assets::balance(0, 1), 50);
			assert_eq!(Assets::balance(0, 2), 50);
		});
	}

	#[test]
	fn transferring_amount_less_than_available_balance_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			assert_ok!(Assets::issue(Origin::signed(1), 100));
			assert_eq!(Assets::balance(0, 1), 100);
			assert_ok!(Assets::transfer(Origin::signed(1), 0, 2, 50));
			assert_eq!(Assets::balance(0, 1), 50);
			assert_eq!(Assets::balance(0, 2), 50);
			assert_ok!(Assets::destroy(Origin::signed(1), 0));
			assert_eq!(Assets::balance(0, 1), 0);
			assert_noop!(Assets::transfer(Origin::signed(1), 0, 1, 50), "origin account balance must be greater than or equal to the transfer amount");
		});
	}

	#[test]
	fn transferring_less_than_one_unit_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			assert_ok!(Assets::issue(Origin::signed(1), 100));
			assert_eq!(Assets::balance(0, 1), 100);
			assert_noop!(Assets::transfer(Origin::signed(1), 0, 2, 0), "transfer amount should be non-zero");
		});
	}

	#[test]
	fn transferring_more_units_than_total_supply_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			assert_ok!(Assets::issue(Origin::signed(1), 100));
			assert_eq!(Assets::balance(0, 1), 100);
			assert_noop!(Assets::transfer(Origin::signed(1), 0, 2, 101), "origin account balance must be greater than or equal to the transfer amount");
		});
	}

	#[test]
	fn destroying_asset_balance_with_positive_balance_should_work() {
		with_externalities(&mut new_test_ext(), || {
			assert_ok!(Assets::issue(Origin::signed(1), 100));
			assert_eq!(Assets::balance(0, 1), 100);
			assert_ok!(Assets::destroy(Origin::signed(1), 0));
		});
	}

	#[test]
	fn destroying_asset_balance_with_zero_balance_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			assert_ok!(Assets::issue(Origin::signed(1), 100));
			assert_eq!(Assets::balance(0, 2), 0);
			assert_noop!(Assets::destroy(Origin::signed(2), 0), "origin balance should be non-zero");
		});
	}
}
