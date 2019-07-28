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

use serde::{Deserialize, Serialize};
use primitives::U256;
use runtime_primitives::traits;

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum NumberOrHex<Number> {
    /// The original header number type of block.
    Number(Number),
    /// Hex representation of the block number.
    Hex(U256),
}

impl<Number: traits::As<u64>> NumberOrHex<Number> {
    /// Attempts to convert into concrete block number.
    ///
    /// Fails in case hex number is too big.
    pub fn to_number(self) -> Result<Number, String> {
        match self {
            NumberOrHex::Number(n) => Ok(n),
            NumberOrHex::Hex(h) => {
                // FIXME #1377 this only supports `u64` since `BlockNumber`
                // is `As<u64>` we could possibly go with `u128`.
                let l = h.low_u64();
                if U256::from(l) != h {
                    Err(format!("`{}` does not fit into the block number type.", h))
                } else {
                    Ok(traits::As::sa(l))
                }
            },
        }
    }
}