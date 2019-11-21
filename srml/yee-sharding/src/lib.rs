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
    inherents::{
        InherentDataProviders, ProvideInherentData,
    },
};
use {
    inherents::{
        InherentData, InherentIdentifier,
        MakeFatalError, ProvideInherent, RuntimeString,
    },
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
        storage::StorageValue,
    },
    system::{self, ensure_inherent},
    sharding_primitives::ShardingInfo,
};

pub type Log<T> = RawLog<<T as Trait>::ShardNum, <T as system::Trait>::BlockNumber>;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"YeeShard";

pub type InherentType = ShardInfo<u16>;

#[derive(Clone, PartialEq, Eq)]
#[derive(Decode, Encode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize))]
pub struct ShardInfo<N> {
    pub num: N,
    pub count: N,
    pub scale_out: Option<ScaleOut<N>>,
}

#[derive(Clone, PartialEq, Eq)]
#[derive(Decode, Encode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize))]
pub struct ScaleOut<N> {
    pub shard_num: N,
}

pub trait YeeShardInherentData {
    fn yee_shard_inherent_data(&self) -> Result<InherentType, RuntimeString>;
    fn yee_shard_replace_inherent_data(&mut self, new: InherentType);
}

impl YeeShardInherentData for InherentData {
    fn yee_shard_inherent_data(&self) -> Result<InherentType, RuntimeString> {
        self.get_data(&INHERENT_IDENTIFIER)
            .and_then(|r| r.ok_or_else(|| "YeeShard inherent data not found".into()))
    }

    fn yee_shard_replace_inherent_data(&mut self, new: InherentType) {
        self.replace_data(INHERENT_IDENTIFIER, &new);
    }
}

#[cfg(feature = "std")]
pub struct InherentDataProvider {
    shard_info: ShardInfo<u16>,
}

#[cfg(feature = "std")]
impl InherentDataProvider {
    pub fn new(num: u16, count: u16, scale_out: Option<ScaleOut<u16>>) -> Self {
        Self {
            shard_info: ShardInfo { num, count, scale_out },
        }
    }
}

#[cfg(feature = "std")]
impl ProvideInherentData for InherentDataProvider {
    fn inherent_identifier(&self) -> &'static [u8; 8] {
        &INHERENT_IDENTIFIER
    }

    fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), RuntimeString> {
        inherent_data.put_data(INHERENT_IDENTIFIER, &self.shard_info)
    }

    fn error_to_string(&self, error: &[u8]) -> Option<String> {
        RuntimeString::decode(&mut &error[..]).map(Into::into)
    }
}

/// Logs in this module.
#[cfg_attr(feature = "std", derive(Serialize, Debug))]
#[derive(Encode, Decode, PartialEq, Eq, Clone)]
pub enum RawLog<ShardNum, BlockNumber> {
    /// Block Header digest log for shard info
    ShardMarker(ShardNum, ShardNum),
    ScaleOutPhase(ScaleOutPhase<BlockNumber, ShardNum>),
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

#[derive(Clone, PartialEq, Eq)]
#[derive(Decode, Encode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize))]
pub enum ScaleOutPhase<BlockNumber, ShardNum>{
    Started{
        observe_util: BlockNumber,
        shard_num: ShardNum,
    },
    NativeReady{
        observe_util: BlockNumber,
        shard_num: ShardNum,
    },
    Ready{
        observe_util: BlockNumber,
        shard_num: ShardNum,
    },
    Commiting {
        shard_count: ShardNum,
    },
    Committed {
        shard_num: ShardNum,
        shard_count: ShardNum,
    },
}

decl_storage! {
    trait Store for Module<T: Trait> as Sharding {
        /// Total sharding count used in genesis block
        pub GenesisShardingCount get(genesis_sharding_count) config(): T::ShardNum;

        /// Total sharding count used in genesis block
        pub ScaleOutObserveBlocks get(scale_out_observe_blocks) config(): T::BlockNumber;

        /// Storage for ShardInfo used for current block
        pub CurrentShardInfo get(current_shard_info): Option<ShardInfo<T::ShardNum>>;

        /// Storage for ScaleOutPhase used for current block
        pub CurrentScaleOutPhase get(current_scale_out_phase): Option<ScaleOutPhase<T::BlockNumber, T::ShardNum>>;

    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn set_shard_info(origin, info: ShardInfo<T::ShardNum>) {
            ensure_inherent(origin)?;

            let info_clone = info.clone();
            <Self as Store>::CurrentShardInfo::mutate(|orig| {
                *orig = Some(info_clone);
            });

            let block_number = <system::Module<T>>::block_number();
            let scale_out_observe_blocks = Self::scale_out_observe_blocks();

            let current_scale_out_phase = Self::current_scale_out_phase();

            let target_shard_num = match info.scale_out.clone(){
                Some(scale_out) => scale_out.shard_num,
                None => info.num,
            };

            match current_scale_out_phase {
                None => {
                    if let Some(scale_out) = info.scale_out {
                        <Self as Store>::CurrentScaleOutPhase::mutate(|orig| {
                            *orig = Some(ScaleOutPhase::Started{
                                observe_util: block_number + scale_out_observe_blocks,
                                shard_num: target_shard_num,
                            });
                        });
                    }
                },
                Some(current_scale_out_phase) => match current_scale_out_phase{
                    ScaleOutPhase::Started{observe_util, shard_num} => {

                        //TODO: check scaled shard_num percentage
                        if observe_util == block_number{
                            <Self as Store>::CurrentScaleOutPhase::mutate(|orig| {
                                *orig = Some(ScaleOutPhase::NativeReady{
                                    observe_util: block_number + scale_out_observe_blocks,
                                    shard_num: target_shard_num,
                                });
                            });
                        }
                    },
                    ScaleOutPhase::NativeReady{observe_util, shard_num} => {

                        //TODO: check foreign scale out phase
                        if observe_util == block_number{
                            <Self as Store>::CurrentScaleOutPhase::mutate(|orig| {
                                *orig = Some(ScaleOutPhase::Ready{
                                    observe_util: block_number + scale_out_observe_blocks,
                                    shard_num: target_shard_num,
                                });
                            });
                        }

                    },
                    ScaleOutPhase::Ready{observe_util, shard_num} => {

                        if observe_util == block_number{

                            let scale_out_shard_count = info.count + info.count;

                            <Self as Store>::CurrentScaleOutPhase::mutate(|orig| {
                                *orig = Some(ScaleOutPhase::Commiting{
                                    shard_count: scale_out_shard_count,
                                });
                            });
                        }
                    },
                    ScaleOutPhase::Commiting{shard_count} => {

                        <Self as Store>::CurrentScaleOutPhase::mutate(|orig| {
                            *orig = Some(ScaleOutPhase::Committed{
                                shard_num: target_shard_num,
                                shard_count: shard_count,
                            });
                        });
                    },
                    ScaleOutPhase::Committed{shard_num, shard_count} => {

                        <Self as Store>::CurrentScaleOutPhase::mutate(|orig| {
                            *orig = None;
                        });
                    },
                }
            }

        }

        fn on_finalize(block_number: T::BlockNumber) {

            if let Some(shard_info) = Self::current_shard_info() {
                Self::deposit_log(RawLog::ShardMarker(shard_info.num, shard_info.count));
            }

            if let Some(scale_out_phase) = Self::current_scale_out_phase() {
                Self::deposit_log(RawLog::ScaleOutPhase(scale_out_phase));
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

impl<T: Trait> ShardingInfo<T::ShardNum> for Module<T> {
    fn get_genesis_shard_count() -> <T as Trait>::ShardNum {
        Self::genesis_sharding_count()
    }

    fn get_curr_shard() -> Option<T::ShardNum> {
        Some(Self::current_shard_info()
            .expect("shard info must be ready for runtime modules")
            .num
        )
    }

    fn get_shard_count() -> T::ShardNum {
        Self::current_shard_info()
            .expect("shard info must be ready for runtime modules")
            .count
    }
}

impl<T: Trait> ProvideInherent for Module<T> {
    type Call = Call<T>;
    type Error = MakeFatalError<RuntimeString>;
    const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

    fn create_inherent(data: &InherentData) -> Option<Self::Call> {
        let data = extract_inherent_data::<T::ShardNum>(data)
            .expect("Sharding inherent data must exist");

        Some(Call::set_shard_info(data))
    }

    fn check_inherent(_: &Self::Call, _: &InherentData) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn extract_inherent_data<N>(data: &InherentData) -> Result<ShardInfo<N>, RuntimeString> where
    N: Decode,
{
    data.get_data::<ShardInfo<N>>(&INHERENT_IDENTIFIER)
        .map_err(|_| RuntimeString::from("Invalid sharding inherent data encoding."))?
        .ok_or_else(|| "Sharding inherent data is not provided.".into())
}
