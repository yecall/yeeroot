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

/// (chain_spec_id, shard_num) => (block_number, block_hash, fork_id)
pub const FORK_CONF : [((&str, u16), (u64, &str, u32)); 4] = [
    (("mainnet", 0),  ( 144005, "0x3c6a4cbfd3e178967aa389db39d8299078471a9a02285a159d78be30334e799e", 1)),
    (("mainnet", 1),  ( 144063, "0x7bdb8b504814ecba70a6d62e312d37ed1c4c08e31cff83d0c6eea81468fde7de", 1)),
    (("mainnet", 2),  ( 144052, "0xfbee5fa7822dfef5326aba0cc7e9e735e7e2520e81f5c64e86b6e28007162a16", 1)),
    (("mainnet", 3),  ( 142548, "0xf79d80730adae46fa1c31893554d2662f83b2ea886bd0df0b406612504728c45", 1)),
];
