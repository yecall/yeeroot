#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use serde::Serialize;
use {
    runtime_primitives::{
        codec::{Decode, Encode},
    },
    support::decl_module,
};

pub type Log<T> = RawLog<<T as system::Trait>::Hash>;

/// Logs in this module.
#[cfg_attr(feature = "std", derive(Serialize, Debug))]
#[derive(Encode, Decode, PartialEq, Eq, Clone)]
pub enum RawLog<Hash> {
    /// Block Header digest log for shard info
    RelayProof(Hash),
}

pub trait Trait: system::Trait {
    /// Type for all log entries of this module.
    type Log: From<Log<Self>> + Into<system::DigestItemOf<Self>>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}