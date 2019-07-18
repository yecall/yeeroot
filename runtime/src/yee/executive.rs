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

use {
    rstd::marker::PhantomData,
    runtime_primitives::traits::{
        self, NumberFor,
    },
};
use {
    consensus_pow::DifficultyType,
};

pub struct Executive<System, Block> (
    PhantomData<(System, Block)>,
);

impl<System, Block> Executive<System, Block> where
    System: system::Trait,
    Block: traits::Block<Header=System::Header, Hash=System::Hash>,
{
    pub fn calc_difficulty() -> DifficultyType {
        primitives::U256::from(0x0000ffff) << 224
    }
}
