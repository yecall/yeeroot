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

#[cfg(feature = "std")]
use {
    serde::Serialize,
};
use {
    runtime_primitives::{
        codec::{
            Codec, Decode, Encode,
        },
        traits::{
            Member, SimpleArithmetic,
            MaybeDisplay, MaybeSerializeDebug,
        },
    },
    support::{
        decl_module, decl_storage,
    },
};

pub type Log<T> = RawLog<<T as Trait>::ShardNum>;

/// Logs in this module.
#[cfg_attr(feature = "std", derive(Serialize, Debug))]
#[derive(Encode, Decode, PartialEq, Eq, Clone)]
pub enum RawLog<N> {
    /// Block Header digest log for shard info
    ShardMarker(N),
}

pub trait Trait: system::Trait {
    /// Type for shard number
    type ShardNum: Member + MaybeSerializeDebug + Default + Copy + MaybeDisplay + SimpleArithmetic + Codec;
    /// Type for all log entries of this module.
    type Log: From<Log<Self>> + Into<system::DigestItemOf<Self>>;
}

/*
#[cfg(any(feature = "std", test))]
impl<N> From<RawLog<N>> for runtime_primitives::testing::DigestItem {
    fn from(log: RawLog<N>) -> Self {
        match log {
            RawLog::ShardMarker(shard) => {
                runtime_primitives::generic::DigestItem::Other(format!("YeeShard: {:?}", shard).encode())
            }
        }
    }
}
*/

decl_storage! {
    trait Store for Module<T: Trait> as Sharding {
        /// Shard number for this chain
        /// may be not configured, and generated in generated block One
        pub CurrentShard get(current_shard): Option<T::ShardNum>;

        /// Total sharding count, configured from genesis block
        pub ShardingCount get(sharding_count) config(): T::ShardNum;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn on_finalize() {
            if let Some(current_shard) = Self::current_shard() {
                Self::deposit_log(RawLog::ShardMarker(current_shard));
            }
        }
    }
}

impl<T: Trait> Module<T> {
    /// Deposit one of this module's logs.
    fn deposit_log(log: Log<T>) {
        <system::Module<T>>::deposit_log(<T as Trait>::Log::from(log).into());
    }
}
