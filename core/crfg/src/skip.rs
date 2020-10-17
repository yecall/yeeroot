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

/// (chain_spec_id, shard_num) => (block_number, block_hash, skip_number)
pub const SKIP_CONF : [((&str, u16), (u64, &str, u32)); 1] = [
    (("mainnet", 1),  ( 160624, "0xdd62ef02370b79de9160604a4cd6234ac68651a4efc673bf9aae7e0cfc77c348", 160593)),
];