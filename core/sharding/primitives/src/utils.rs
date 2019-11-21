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
    use primitives::sr25519;
    use primitives::sr25519::Pair;
    use primitives::crypto::{Ss58Codec, Pair as PairTrait};
    use yee_primitives::{Address, AddressCodec, Hrp};
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

        assert_eq!(shard_num_for_bytes(&bytes, 0u16), None);

        assert_eq!(shard_num_for_bytes(&bytes, 1u16), Some(0));

        assert_eq!(shard_num_for_bytes(&bytes, 2u16), Some(1));

        assert_eq!(shard_num_for_bytes(&bytes, 4u16), Some(1));

        assert_eq!(shard_num_for_bytes(&bytes, 8u16), Some(0b101));

        assert_eq!(shard_num_for_bytes(&bytes, 16u16), Some(0b1101));
    }

    #[test]
    fn test_address(){

        let address = Address("tyee15c2cc2uj34w5jkfzxe4dndpnngprxe4nytaj9axmzf63ur4f8awq806lv6".to_string());
        let (public, hrp) = AccountId::from_address(&address).map_err(|e| format!("{:?}", e)).unwrap();

        assert_eq!(shard_num_for(&public, 4u16), Some(0));

        // address shard num will change on sharding scale out
        assert_eq!(shard_num_for(&public, 8u16), Some(4));

        let address = Address("tyee10n605lxn7k7rfm4t9nx3jd6lu790m30hs37j7dvm6jeun2kkfg7sf6fp9j".to_string());
        let (public, hrp) = AccountId::from_address(&address).map_err(|e| format!("{:?}", e)).unwrap();

        assert_eq!(shard_num_for(&public, 4u16), Some(1));

        // address shard num will change on sharding scale out
        assert_eq!(shard_num_for(&public, 8u16), Some(5));

        let address = Address("tyee16pa6aa7qnf6w5ztqdvla6kvmeg78pkmpd76d98evl88ppmarcctqdz5nu3".to_string());
        let (public, hrp) = AccountId::from_address(&address).map_err(|e| format!("{:?}", e)).unwrap();

        assert_eq!(shard_num_for(&public, 4u16), Some(2));

        // address shard num will change on sharding scale out
        assert_eq!(shard_num_for(&public, 8u16), Some(6));

        let address = Address("tyee12n2pjuwa5hukpnxjt49q5fal7m5h2ddtxxlju0yepzxty2e2fads5g57yd".to_string());
        let (public, hrp) = AccountId::from_address(&address).map_err(|e| format!("{:?}", e)).unwrap();

        assert_eq!(shard_num_for(&public, 4u16), Some(3));

        // address shard num will not change on sharding scale out
        assert_eq!(shard_num_for(&public, 8u16), Some(3));


    }

    #[test]
    fn test_generate_address() {

        let address = Address("tyee1yjn7neelq90y6x0jxcqavm0hv8gskxs6ctsftdjel4vwq0ecrvgqnh5efm".to_string());
        let (public, hrp) = AccountId::from_address(&address).map_err(|e| format!("{:?}", e)).unwrap();

        assert_eq!(shard_num_for(&public, 4u16), Some(0));

        // address shard num will not change on sharding scale out
        assert_eq!(shard_num_for(&public, 8u16), Some(0));

//        loop {
//            let accountId = Pair::generate().public();
//
//            if shard_num_for(&accountId, 4u16) != Some(0) {
//                continue;
//            }
//
//            if shard_num_for(&accountId, 8u16) != Some(0) {
//                continue;
//            }
//
//            let address = accountId.to_address(Hrp::TESTNET).unwrap();
//
//            assert_eq!(address, Address("".to_string()));
//            break;
//        }

    }

    #[test]
    fn test_fail() {

        let bytes = hex::decode("7d").unwrap();

        let a = shard_num_for_bytes(&bytes, 8u16);

        assert_eq!(a, None);
    }
}
