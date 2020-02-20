// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use rstd::vec::Vec;
use system::ensure_signed;
use parity_codec::Codec;
use srml_support::{decl_module, decl_event, dispatch::Result};
use yee_sr_primitives::MAX_STORAGE_SIZE;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        pub fn store(origin, data: Vec<u8>) -> Result {
            let origin = ensure_signed(origin)?;
            let size = data.len();
            if size > MAX_STORAGE_SIZE {
                return Err("storage size is larger than max size.")
            }
            Self::deposit_event(RawEvent::Storage(origin, size as u32));
            Ok(())
        }
    }
}

decl_event!(
    pub enum Event<T> where <T as system::Trait>::AccountId {
		/// A new storage record event.
		Storage(AccountId, u32),
	}
);
