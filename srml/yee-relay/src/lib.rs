#![cfg_attr(not(feature = "std"), no_std)]

 use parity_codec::{Decode, Encode, Compact};
 use rstd::vec::Vec;
 use rstd::prelude::*;
use srml_support::{decl_module, dispatch::Result};
 use yee_sr_primitives::{RelayTypes, RelayParams};

pub trait Trait: system::Trait {
    // type Number: Default + Clone;
    type Balances: balances::Trait;

    type Assets: assets::Trait;
}

decl_module!{
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        pub fn transfer(relay_type: RelayTypes, tx: Vec<u8>, number: Compact<u64>, hash: T::Hash, parent: T::Hash) -> Result{
            match relay_type {
                RelayTypes::Balance => {
                    <balances::Module<T::Balances>>::relay_transfer(tx)
                },
                RelayTypes::Assets => {
                    <assets::Module<T::Assets>>::relay_transfer(tx)
                }
            }
        }
    }
}
