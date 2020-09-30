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
    (("mainnet", 0),  ( 143054, "0x32e65deeaf15cd7b8f308c6b065aab06956f804ef0279506876adf98f99e4db2", 1)),
    (("mainnet", 1),  ( 143104, "0x841eef9a70ce91fd2a740eb1dc91795111154caa77897596317a1ac3d022ebef", 1)),
    (("mainnet", 2),  ( 143088, "0x1998d592983ecc9285df16d1c0994e8310ad3c0bb362915d11d1eb7f473c2054", 1)),
    (("mainnet", 3),  ( 142547, "0x875315ab95daa46d48818ef366b426b68c78df01949882655a35abb1b97bc021", 1)),
];
