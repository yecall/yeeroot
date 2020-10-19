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

use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    /// (chain_spec_id, shard_num, block_number) => (block_hash, skip_number)
    pub static ref SKIP_CONF: HashMap<(&'static str, u16, u64), (&'static str, u64)> = {
        let mut m = HashMap::new();
        m.insert(("mainnet", 0, 155699), ("0x80efb2876ef10c381bbb7193824b40b1177922234009b147eb6394d176ceb05a", 155673));
        m.insert(("mainnet", 0, 160595), ("0x544ebd2fce52a00375994984b8ff89032a61716e8f2b383c0ce9efaeeaec79a5", 160564));

        m.insert(("mainnet", 1, 160624), ("0xdd62ef02370b79de9160604a4cd6234ac68651a4efc673bf9aae7e0cfc77c348", 160593));

        m.insert(("mainnet", 2, 160503), ("0x22eb3b842f3a8ad5a3445037688ec36f5330e59720cc4d724f59115d6916b82c", 160472));
        m.insert(("mainnet", 2, 165579), ("0x5501ca5222b478f7fdf8428a32ca2eb18da931379af0d569065a6ba2f7532c1a", 165520));

        m.insert(("mainnet", 3, 158788), ("0x7761c0ca109e9b2a251cdcaac0e328b35b5ce781a9f21b18f6666494aa5168f4", 158757));
        m
    };
}
