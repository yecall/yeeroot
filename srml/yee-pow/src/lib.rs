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

#[cfg(feature = "std")]
use {
    serde::Serialize,
    inherents::{
        InherentDataProviders, ProvideInherentData,
    },
};
use {
    inherents::{
        InherentData, InherentIdentifier,
        MakeFatalError, ProvideInherent, RuntimeString,
    },
    srml_support::{
        decl_module, decl_storage,
        Parameter,
        storage::StorageValue,
    },
    primitives::{
        codec::{
            Codec, Decode, Encode,
        },
    },
    system::ensure_inherent,
};
use rstd::{result, prelude::*};

pub trait Trait: system::Trait {
    /// Type used for block difficulty
    type Difficulty: Parameter + Default;
}

decl_storage! {
    trait Store for Module<T: Trait> as Pow {
        /// Block POW Difficulty
        pub GenesisDifficulty get(genesis_difficulty) config(): T::Difficulty;

        /// Difficulty adjust period in block number
        pub DifficultyAdj get(difficulty_adj) config(): T::BlockNumber;

        /// Target block time in seconds
        pub TargetBlockTime get(target_block_time) config(): u64;

        /// Storage for PowInfo for current block
        pub CurrentPowInfo get(current_pow_info): Option<PowInfo<T::AccountId>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        fn set_pow_info(origin, info: PowInfo<T::AccountId>) {
            ensure_inherent(origin)?;

            <Self as Store>::CurrentPowInfo::mutate(|orig| {
                *orig = Some(info);
            });
        }

    }
}

impl<T: Trait> Module<T> {
    //
}

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"YeePow00";

pub type InherentType<AccountId> = PowInfo<AccountId>;

#[derive(Clone, PartialEq, Eq)]
#[derive(Decode, Encode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize))]
pub struct PowInfo<AccountId> {
    pub coinbase: AccountId,
}

pub trait PowInherentData<AccountId> {
    fn pow_inherent_data(&self) -> Result<InherentType<AccountId>, RuntimeString>;
    fn pow_replace_inherent_data(&mut self, new: InherentType<AccountId>);
}

impl<AccountId: Codec> PowInherentData<AccountId> for InherentData {
    fn pow_inherent_data(&self) -> Result<InherentType<AccountId>, RuntimeString> {
        self.get_data(&INHERENT_IDENTIFIER)
            .and_then(|r| r.ok_or_else(|| "YeePow inherent data not found".into()))
    }

    fn pow_replace_inherent_data(&mut self, new: InherentType<AccountId>) {
        self.replace_data(INHERENT_IDENTIFIER, &new);
    }
}

#[cfg(feature = "std")]
pub struct InherentDataProvider<AccountId> {
    pow_info: PowInfo<AccountId>,
}

#[cfg(feature = "std")]
impl<AccountId> InherentDataProvider<AccountId> {
    pub fn new(coinbase: AccountId) -> Self {
        Self {
            pow_info: PowInfo { coinbase },
        }
    }
}

#[cfg(feature = "std")]
impl<AccountId: Codec> ProvideInherentData for InherentDataProvider<AccountId> {

    fn on_register(
        &self,
        providers: &InherentDataProviders,
    ) -> result::Result<(), RuntimeString> {
        if !providers.has_provider(&srml_timestamp::INHERENT_IDENTIFIER) {
            // Add the timestamp inherent data provider, as we require it.
            providers.register_provider(srml_timestamp::InherentDataProvider)
        } else {
            Ok(())
        }
    }

    fn inherent_identifier(&self) -> &'static [u8; 8] {
        &INHERENT_IDENTIFIER
    }

    fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), RuntimeString> {
        inherent_data.put_data(INHERENT_IDENTIFIER, &self.pow_info)
    }

    fn error_to_string(&self, error: &[u8]) -> Option<String> {
        RuntimeString::decode(&mut &error[..]).map(Into::into)
    }
}

impl<T: Trait> ProvideInherent for Module<T> {
    type Call = Call<T>;
    type Error = MakeFatalError<RuntimeString>;
    const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

    fn create_inherent(data: &InherentData) -> Option<Self::Call> {

        let data : PowInfo<T::AccountId> = data.pow_inherent_data().expect("Yee pow inherent data must exist");

        Some(Call::set_pow_info(data))
    }

    fn check_inherent(_: &Self::Call, _: &InherentData) -> Result<(), Self::Error> {
        Ok(())
    }
}
