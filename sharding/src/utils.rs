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

use yee_runtime::AccountId;

pub fn shard_num_for_account_id(account_id: &AccountId, shard_count: u16) -> Option<u16> {

    shard_num_for_bytes(&account_id.as_slice(), shard_count)
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
    let digits = (shard_count as f64).log2() as u16;

    if (2f64.powf(digits as f64) as u16) == shard_count {
        Some(digits)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use substrate_primitives::sr25519::Public;
    use yee_runtime::AccountId;
    use crate::utils::shard_num_for_account_id;
    use crate::utils::shard_num_for_bytes;

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

        let a = shard_num_for_account_id(&account_id, 0u16);

        assert_eq!(a, None);

        let a = shard_num_for_account_id(&account_id, 1u16);

        assert_eq!(a, Some(0));

        let a = shard_num_for_account_id(&account_id, 2u16);

        assert_eq!(a, Some(1));

        let a = shard_num_for_account_id(&account_id, 4u16);

        assert_eq!(a, Some(1));

        let a = shard_num_for_account_id(&account_id, 8u16);

        assert_eq!(a, Some(0b101));

        let a = shard_num_for_account_id(&account_id, 16u16);

        assert_eq!(a, Some(0b1101));
    }

    #[test]
    fn test_fail() {

        let bytes = hex::decode("7d").unwrap();

        let a = shard_num_for_bytes(&bytes, 8u16);

        assert_eq!(a, None);
    }
}
