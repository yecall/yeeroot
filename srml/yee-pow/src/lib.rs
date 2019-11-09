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
        traits::{
            Currency, OnUnbalanced, Imbalance,
        }
    },
    primitives::{
        codec::{
            Codec, Decode, Encode,
        },
    },
    system::ensure_inherent,
};
use rstd::{result, prelude::*};

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
type PositiveImbalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::PositiveImbalance;

pub trait Trait: system::Trait {
    /// Type used for block difficulty
    type Difficulty: Parameter + Default;

    /// The reward balance.
    type Currency: Currency<Self::AccountId>;

    /// Handler for the unbalanced increment when rewarding a staker.
    type Reward: OnUnbalanced<PositiveImbalanceOf<Self>>;
}

#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize))]
pub struct RewardPlan<N, AccountId, Balance> {
    block_number: N,
    coinbase: AccountId,
    block_reward: Balance,
    fee_reward: Balance,
}

decl_storage! {
    trait Store for Module<T: Trait> as Pow {
        /// Block POW Difficulty
        pub GenesisDifficulty get(genesis_difficulty) config(): T::Difficulty;

        /// Difficulty adjust period in block number
        pub DifficultyAdj get(difficulty_adj) config(): T::BlockNumber;

        /// Target block time in seconds
        pub TargetBlockTime get(target_block_time) config(): u64;

        /// Block reward
        pub BlockReward get(block_reward) config(): BalanceOf<T>;

        /// Block reward latency
        pub BlockRewardLatency get(block_reward_latency) config(): T::BlockNumber;

        /// Storage for PowInfo for current block
        pub RewardPlans get(reward_plans): Vec<RewardPlan<T::BlockNumber, T::AccountId, BalanceOf<T>>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        fn set_pow_info(origin, info: PowInfo<T::AccountId>) {
            ensure_inherent(origin)?;

            let reward_condition = info.reward_condition;

            let block_number = <system::Module<T>>::block_number();
            let coinbase = info.coinbase.clone();
            let block_reward = Self::block_reward();
            let fee_reward = Default::default();

            let new_reward_plan = RewardPlan{
                block_number,
                coinbase: coinbase.clone(),
                block_reward,
                fee_reward,
            };

            let reward_block_number = if block_number >= Self::block_reward_latency() {
                block_number - Self::block_reward_latency()
            } else{
                Default::default()
            };

            <Self as Store>::RewardPlans::mutate(|orig| {
                orig.push(new_reward_plan);

                orig.retain(|x|{
                    let reward = x.block_number <= reward_block_number;
                    if reward{
                        Self::reward(x, coinbase.clone(), reward_condition.clone());
                    }
                    !reward
                });

            });
        }

    }
}

impl<T: Trait> Module<T> {

    fn reward(reward_plan: &RewardPlan<T::BlockNumber, T::AccountId, BalanceOf<T>>, current_coinbase: T::AccountId, reward_condition: RewardCondition){
        let reward_target = match reward_condition{
            RewardCondition::Normal => &reward_plan.coinbase,
            RewardCondition::Slash => &current_coinbase,
        };
        let reward_amount = reward_plan.block_reward + reward_plan.fee_reward;
        let imbalance = T::Currency::deposit_creating(reward_target, reward_amount);
        T::Reward::on_unbalanced(imbalance);
    }
}

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"YeePow00";

pub type InherentType<AccountId> = PowInfo<AccountId>;

#[derive(Clone, PartialEq, Eq)]
#[derive(Decode, Encode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize))]
pub struct PowInfo<AccountId> {
    pub coinbase: AccountId,
    pub reward_condition: RewardCondition,
}

#[derive(Clone, PartialEq, Eq)]
#[derive(Decode, Encode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize))]
pub enum RewardCondition {
    Normal,
    Slash,//TODO provide slash reason
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
    pub fn new(coinbase: AccountId, reward_condition: RewardCondition) -> Self {
        Self {
            pow_info: PowInfo { coinbase, reward_condition },
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
