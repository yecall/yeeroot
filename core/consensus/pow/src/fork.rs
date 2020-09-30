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
    (("mainnet", 0),  ( 143459, "0x473de2d82aebbefe0e96888d1fde3a7c206f6842aaaafc546fecac4a62f760e1", 1)),
    (("mainnet", 1),  ( 143514, "0x55867d252b5c8e3eaa54b1ea865e7db035adcb3b6cda8b0fce0a4964bd8a0469", 1)),
    (("mainnet", 2),  ( 143501, "0x3f39ed1bce2eb79712d05ad799349a66bedc634d764bbed83e800594e3a0f692", 1)),
    (("mainnet", 3),  ( 141721, "0xee73f763ef9f55a7e85d118026ce62a26597a9c246c914e26a29309242e8c065", 1)),
];
