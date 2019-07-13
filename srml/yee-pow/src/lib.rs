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

#![cfg_attr(not(feature = "std"), no_std)]

///! Consensus extension module for POW consensus.

use {
    srml_support::{
        decl_module, decl_storage,
        Parameter,
    },
};


pub trait Trait: system::Trait {
    /// Type used for block difficulty
    type Difficulty: Parameter + Default;
}

decl_storage! {
    trait Store for Module<T: Trait> as Pow {
        /// Block POW Difficulty
        pub Difficulty get(difficulty) config(): T::Difficulty;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin { }
}

impl<T: Trait> Module<T> {
    //
}
