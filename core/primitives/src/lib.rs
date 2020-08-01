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

use bech32::{ToBase32, Error, FromBase32};
use std::fmt;
use serde::Deserialize;
use parity_codec::{Encode, Decode};
use parity_codec::alloc::collections::HashMap;

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Hrp {
	MAINNET,
	TESTNET,
}

impl Default for Hrp{

	fn default() -> Self{
		Hrp::MAINNET
	}
}

#[derive(Clone, Eq, PartialEq, Debug, Deserialize)]
pub struct Address(pub String);

pub trait AddressCodec: Sized {
	/// Decode from address format
	fn from_address(address: &Address) -> Result<(Self, Hrp), Error>;
	/// Encode to address format
	fn to_address(&self, hrp: Hrp) -> Result<Address, Error>;
}

impl<T: AsMut<[u8]> + AsRef<[u8]> + Default> AddressCodec for T {
	fn from_address(address: &Address) -> Result<(Self, Hrp), Error> {

		let (hrp_str, buf) = bech32::decode(&address.0)?;
		let buf = Vec::from_base32(&buf)?;

		let mut res = T::default();
		res.as_mut().copy_from_slice(&buf);

		let hrp : Hrp = hrp_str.into();

		Ok((res, hrp))

	}
	fn to_address(&self, hrp: Hrp) -> Result<Address, Error> {
		let buf = self.to_base32();

		let hrp_str : String  = hrp.into();
		bech32::encode(
			&hrp_str,
			buf,
		).map(|s| Address(s))
	}
}

impl fmt::Display for Address{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result{
		self.0.fmt(f)
	}
}

///https://github.com/satoshilabs/slips/blob/master/slip-0173.md

const YEE : &str = "yee";
const TYEE : &str = "tyee";

impl Into<String> for Hrp{

	fn into(self) -> String{
		match self{
			Hrp::MAINNET => YEE.to_string(),
			Hrp::TESTNET => TYEE.to_string(),
		}
	}
}

impl From<String> for Hrp{
	fn from(s: String) -> Self{
		let s : &str = &s;
		match s{
			YEE => Hrp::MAINNET,
			TYEE => Hrp::TESTNET,
			_ => unreachable!(),
		}
	}
}

#[derive(Clone, Debug)]
pub struct Config{
	pub shards: HashMap<String, Shard>,
}

#[derive(Clone, Debug)]
pub struct Shard {
	pub rpc: Vec<String>,
}

impl Config{
	pub fn get_shard_count(&self)->u16{
		self.shards.len() as u16
	}
}

#[derive(Encode, Decode, Clone, Debug)]
pub struct RecommitRelay<Hash> {
	pub hash: Hash,
	pub index: usize
}

#[cfg(test)]
mod tests {
	use crate::AddressCodec;
	use crate::Hrp;
	use crate::Address;
	use substrate_primitives::sr25519::Public;
	use substrate_primitives::crypto::Ss58Codec;

	#[test]
	fn test_to_address() {

		let public = hex::decode("0001020304050607080900010203040506070809000102030405060708090001").expect("qed");

		let address = public.to_address(Hrp::TESTNET).expect("qed");

		assert_eq!(Address("tyee1qqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqs0a78ky".to_string()), address);

		let address = public.to_address(Hrp::MAINNET).expect("qed");

		assert_eq!(Address("yee1qqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsz6e3hh".to_string()), address);

	}

	#[test]
	fn test_from_address() {

		let target_public = hex::decode("0001020304050607080900010203040506070809000102030405060708090001").expect("qed");

		let address = Address("tyee1qqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqs0a78ky".to_string());

		let (public, hrp) = <[u8; 32]>::from_address(&address).expect("qed");

		assert_eq!((target_public.clone(), Hrp::TESTNET), (public.to_vec(), hrp));

		let address = Address("yee1qqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsz6e3hh".to_string());

		let (public, hrp) = <[u8; 32]>::from_address(&address).expect("qed");

		assert_eq!((target_public, Hrp::MAINNET), (public.to_vec(), hrp));


	}

	#[test]
	fn test_ss58_to_address() {

		let public = Public::from_string("5FpUCxXVR5KbQLf3qsfwxzdczyU74VeNYw9ba3rdocn23svG").expect("qed");

		let address = public.to_address(Hrp::TESTNET).expect("qed");

		assert_eq!(Address("tyee15c2cc2uj34w5jkfzxe4dndpnngprxe4nytaj9axmzf63ur4f8awq806lv6".to_string()), address);


		let public = Public::from_string("5EtYZwFsQR2Ex1abqYFsmTxpHWytPkphS1LDsrCJ2Gr6b695").expect("qed");

		let address = public.to_address(Hrp::TESTNET).expect("qed");

		assert_eq!(Address("tyee10n605lxn7k7rfm4t9nx3jd6lu790m30hs37j7dvm6jeun2kkfg7sf6fp9j".to_string()), address);


		let public = Public::from_string("5Gn4ZNCiPGjBrPa7W1DHDCj83u6R9FyUChafM7nTpvW7iHEi").expect("qed");

		let address = public.to_address(Hrp::TESTNET).expect("qed");

		assert_eq!(Address("tyee16pa6aa7qnf6w5ztqdvla6kvmeg78pkmpd76d98evl88ppmarcctqdz5nu3".to_string()), address);


		let public = Public::from_string("5DyvtMHN3G9TvqVp6ZFcmLuJaRjSYibt2Sh5Hb32cNTTHVB9").expect("qed");

		let address = public.to_address(Hrp::TESTNET).expect("qed");

		assert_eq!(Address("tyee12n2pjuwa5hukpnxjt49q5fal7m5h2ddtxxlju0yepzxty2e2fads5g57yd".to_string()), address);
	}

}