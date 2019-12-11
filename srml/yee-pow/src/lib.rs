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
        decl_module, decl_storage, decl_event,
        Parameter,
        storage::StorageValue,
        traits::{
            Currency, OnUnbalanced,
        }
    },
    primitives::{
        codec::{
            Codec, Decode, Encode,
        },
        traits::{
            As,
        }
    },
    system::ensure_inherent,
};
use rstd::{result, prelude::*};
use yee_srml_sharding::{self as sharding};
use yee_sharding_primitives::ShardingInfo;
use yee_sharding_primitives::utils::shard_num_for;

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
type PositiveImbalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::PositiveImbalance;

pub trait Trait: system::Trait + sharding::Trait {
    /// Type used for pow target
    type PowTarget: Parameter + Default;

    /// The reward balance.
    type Currency: Currency<Self::AccountId>;

    /// Handler for the unbalanced increment when rewarding a staker.
    type Reward: OnUnbalanced<PositiveImbalanceOf<Self>>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type Sharding: ShardingInfo<Self::ShardNum>;

}

pub trait OnFeeWithdrawn<Amount> {

    /// Handler for fee withdrawn
    fn on_fee_withdrawn(amount: Amount);
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize))]
pub struct RewardPlan<N, AccountId, Balance> {
    block_number: N,
    coinbase: AccountId,
    block_reward: Balance,
    fee_reward: Balance,
}

decl_storage! {
    trait Store for Module<T: Trait> as Pow {
        /// Genesis POW target
        pub GenesisPowTarget get(genesis_pow_target) config(): T::PowTarget;

        /// POW target adjust period in block number
        pub PowTargetAdj get(pow_target_adj) config(): T::BlockNumber;

        /// Target block time in seconds
        pub TargetBlockTime get(target_block_time) config(): u64;

        /// Block reward
        pub BlockReward get(block_reward) config(): BalanceOf<T>;

        /// Block reward latency
        pub BlockRewardLatency get(block_reward_latency) config(): T::BlockNumber;

        /// Storage for PowInfo for current block
        pub RewardPlans get(reward_plans): Vec<RewardPlan<T::BlockNumber, T::AccountId, BalanceOf<T>>>;

        /// Storage for total fee for current block
        pub TotalFee get(total_fee): BalanceOf<T>;

        /// Storage for total fee for current block
        pub CurrentPowInfo get(current_pow_info): Option<PowInfo<T::AccountId>>;

    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        fn deposit_event<T>() = default;

        fn set_pow_info(origin, info: PowInfo<T::AccountId>) {
            ensure_inherent(origin)?;

            <Self as Store>::CurrentPowInfo::mutate(|orig| {
                *orig = Some(info);
            });

        }

        fn on_initialize(_block_number: T::BlockNumber) {

            <Self as Store>::TotalFee::mutate(|orig| {
                *orig = Default::default();
            });
        }

        fn on_finalize(block_number: T::BlockNumber) {

            if let Some(info) = Self::current_pow_info(){

                let shard_count = <BalanceOf<T> as As<u64>>::sa(
                    T::Sharding::get_shard_count().as_() as u64
                );

                let reward_condition = info.reward_condition;

                let block_number = block_number;
                let coinbase = info.coinbase.clone();
                let block_reward = Self::block_reward() / shard_count;
                let fee_reward = Self::total_fee();

                let new_reward_plan = RewardPlan{
                    block_number,
                    coinbase: coinbase.clone(),
                    block_reward,
                    fee_reward,
                };

                let reward_block_number = if block_number > Self::block_reward_latency() {
                    block_number - Self::block_reward_latency()
                } else{
                    Default::default()
                };

                <Self as Store>::RewardPlans::mutate(|orig| {
                    orig.push(new_reward_plan.clone());

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
}

decl_event!(
	pub enum Event<T> where Balance = BalanceOf<T>, N = <T as system::Trait>::BlockNumber, <T as system::Trait>::AccountId {
		Reward(RewardPlan<N, AccountId, Balance>),
	}
);

impl<T: Trait> Module<T> {

    fn reward(reward_plan: &RewardPlan<T::BlockNumber, T::AccountId, BalanceOf<T>>, current_coinbase: T::AccountId, reward_condition: RewardCondition){

        let shard_num = T::Sharding::get_curr_shard().expect("qed").as_() as u16;
        let shard_count = T::Sharding::get_shard_count().as_() as u16;
        let coinbase_shard_num = shard_num_for(&reward_plan.coinbase, shard_count).expect("qed");

        //when scaling out, only one splitted shard will perform rewarding
        if coinbase_shard_num == shard_num {
            let reward_target = match reward_condition {
                RewardCondition::Normal => &reward_plan.coinbase,
                RewardCondition::Slash => &current_coinbase,
            };
            let reward_amount = reward_plan.block_reward + reward_plan.fee_reward;
            let imbalance = T::Currency::deposit_creating(reward_target, reward_amount);
            T::Reward::on_unbalanced(imbalance);

            Self::deposit_event(RawEvent::Reward(RewardPlan {
                block_number: reward_plan.block_number.clone(),
                coinbase: reward_target.clone(),
                block_reward: reward_plan.block_reward,
                fee_reward: reward_plan.fee_reward,
            }));
        }
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
    Slash,//TODO: provide slash reason
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

    fn check_inherent(_: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
        let d = data.get_data(&INHERENT_IDENTIFIER)
            .and_then(|r| r.ok_or_else(|| "YeePow inherent data not found".into()));
        let d: InherentType<T::AccountId> = match d {
            Ok(d) => d,
            _ => return Ok(())
        };

        let shard = data.get_data(&sharding::INHERENT_IDENTIFIER)
            .and_then(|r| r.ok_or_else(|| "Sharding inherent data not found".into()));
        let shard: sharding::InherentType = match shard {
            Ok(s) => s,
            _ => return Ok(())
        };

        let shard_num = yee_sharding_primitives::utils::shard_num_for(&d.coinbase, shard.count).unwrap();

        if shard_num != shard.num {
            return Err(RuntimeString::from("shard and coinbase not match").into());
        }
        Ok(())
    }
}

impl<T: Trait> OnFeeWithdrawn<BalanceOf<T>> for Module<T> {

    fn on_fee_withdrawn(amount: BalanceOf<T>){

        <Self as Store>::TotalFee::mutate(|orig| {
            *orig = *orig + amount;
        });

    }

}
