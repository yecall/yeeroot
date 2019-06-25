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

//! POW work proof used in block header digest

use parity_codec::{Decode, Encode, Input, Output};

pub enum WorkProof {
}

impl Decode for WorkProof {
    fn decode<I: Input>(value: &mut I) -> Option<Self> {
        unimplemented!()
    }
}

impl Encode for WorkProof {
    fn encode(&self) -> Vec<u8> {
        unimplemented!()
    }
}
