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

use parity_codec::Codec;

pub fn shard_num_for<T: Codec>(t: &T, shard_count: u16) -> Option<u16> {

    let encoded = t.encode();

    shard_num_for_bytes(encoded.as_slice(), shard_count)
}

pub fn shard_num_for_bytes(bytes: &[u8], shard_count: u16) -> Option<u16> {

    let len = bytes.len();

    if len < 2 {
        return None;
    }

    get_digits(shard_count).map(|digits| {

        let a = u16::from_be_bytes([bytes[len-2], bytes[len-1]]);
        let mask = !(!0u16 << digits);
        a & mask
    })
}

fn get_digits(shard_count: u16) -> Option<u16> {
    if shard_count == 0{
        return None;
    }
    let digits = log2(shard_count);

    if pow2(digits) == shard_count {
        Some(digits)
    } else {
        None
    }
}

fn log2(n: u16) -> u16 {
    let mut s = n;
    let mut i = 0;
    while s > 0 {
        s = s >> 1;
        i = i + 1;
    }
    i - 1
}

fn pow2(n: u16) -> u16{
    1u16 << n
}

#[cfg(test)]
mod tests {
    use primitives::sr25519::Public;
    use yee_runtime::AccountId;
    use primitives::crypto::{Ss58Codec, Pair as PairTrait};
    use crate::utils::shard_num_for;
    use crate::utils::shard_num_for_bytes;
    use crate::utils::log2;
    use crate::utils::pow2;

    #[test]
    fn test_log2(){

        assert_eq!(0, log2(1));
        assert_eq!(1, log2(2));
        assert_eq!(2, log2(4));
        assert_eq!(10, log2(1024));

    }

    #[test]
    fn test_pow2(){

        assert_eq!(1, pow2(0));
        assert_eq!(2, pow2(1));
        assert_eq!(4, pow2(2));
        assert_eq!(1024, pow2(10));

    }

    #[test]
    fn test_bytes() {

        let bytes = hex::decode("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d").unwrap();

        let a = shard_num_for_bytes(&bytes, 0u16);

        assert_eq!(a, None);

        let a = shard_num_for_bytes(&bytes, 1u16);

        assert_eq!(a, Some(0));

        let a = shard_num_for_bytes(&bytes, 2u16);

        assert_eq!(a, Some(1));

        let a = shard_num_for_bytes(&bytes, 4u16);

        assert_eq!(a, Some(1));

        let a = shard_num_for_bytes(&bytes, 8u16);

        assert_eq!(a, Some(0b101));

        let a = shard_num_for_bytes(&bytes, 16u16);

        assert_eq!(a, Some(0b1101));
    }

    #[test]
    fn test_account_id() {

        let account_id = Public::from_slice(&hex::decode("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d").unwrap());

        let a = shard_num_for(&account_id, 0u16);

        assert_eq!(a, None);

        let a = shard_num_for(&account_id, 1u16);

        assert_eq!(a, Some(0));

        let a = shard_num_for(&account_id, 2u16);

        assert_eq!(a, Some(1));

        let a = shard_num_for(&account_id, 4u16);

        assert_eq!(a, Some(1));

        let a = shard_num_for(&account_id, 8u16);

        assert_eq!(a, Some(0b101));

        let a = shard_num_for(&account_id, 16u16);

        assert_eq!(a, Some(0b1101));
    }

    #[test]
    fn test_address(){
        let account_id = Public::from_ss58check("5FpUCxXVR5KbQLf3qsfwxzdczyU74VeNYw9ba3rdocn23svG").unwrap();
        //pk: 0xf8eb0d437140e458ec6103965a4442f6b00e37943142017e9856f3310023ab530a0cc96e386686f95d2da0c7fa423ab7b84d5076b3ba6e7756e21aaafe9d3696

        let a = shard_num_for(&account_id, 4u16);

        assert_eq!(a, Some(0));

        let account_id = Public::from_ss58check("5EtYZwFsQR2Ex1abqYFsmTxpHWytPkphS1LDsrCJ2Gr6b695").unwrap();
        //pk: 0xd0542cb78c304aa7ea075c93772d2a8283b75ea218eb9d6dd96ee181fc9da26caa746ccc1625cbd7451c25860c268792f57f108d536034173a42353ced9cf1e1

        let a = shard_num_for(&account_id, 4u16);

        assert_eq!(a, Some(1));

        let account_id = Public::from_ss58check("5Gn4ZNCiPGjBrPa7W1DHDCj83u6R9FyUChafM7nTpvW7iHEi").unwrap();
        //pk: 0xa8f84e392246b1a4317b1deb904a8272c0428d3d324e1889be8f00b0500a1e63845dbc96f4726783d94d7edcdeb8878ce4dcac793c41e815942c664687599c19

        let a = shard_num_for(&account_id, 4u16);

        assert_eq!(a, Some(2));

        let account_id = Public::from_ss58check("5DyvtMHN3G9TvqVp6ZFcmLuJaRjSYibt2Sh5Hb32cNTTHVB9").unwrap();
        //pk: 0xa079ef650520662d08f270c4bc088f0c61abd0224f58243f6d1e6827c3ab234a7a1a0a3b89bbb02f2b10e357fd2a5ddb5050bc528c875a6990874f9dc6496772

        let a = shard_num_for(&account_id, 4u16);

        assert_eq!(a, Some(3));

    }

    #[test]
    fn test_fail() {

        let bytes = hex::decode("7d").unwrap();

        let a = shard_num_for_bytes(&bytes, 8u16);

        assert_eq!(a, None);
    }
}
