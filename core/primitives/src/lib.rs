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

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Hrp {
	MAINNET,
	TESTNET,
}

#[derive(Clone, Eq, PartialEq, Debug)]
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
		println!("{:?}", hrp_str);
		println!("{:?}", buf);
		let buf = Vec::from_base32(&buf)?;

		println!("{:?}", buf);

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

#[cfg(test)]
mod tests {
	use crate::AddressCodec;
	use crate::Hrp;
	use crate::Address;

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
}