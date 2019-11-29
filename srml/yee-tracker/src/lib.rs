// Copyright 2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! SRML module that tracks the last finalized block, as perceived by block authors.

#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate srml_support;

use inherents::{
	RuntimeString, InherentIdentifier, ProvideInherent,
	InherentData, MakeFatalError,
};
use srml_support::StorageValue;
use primitives::traits::{As, One, Zero};
use rstd::{prelude::*, result, cmp, vec};
use system::{ensure_inherent, Trait as SystemTrait};

use primitives::{
	codec::{
		Codec, Decode, Encode,
	},
	traits::{
		Member, SimpleArithmetic,
		MaybeDisplay, MaybeSerializeDebug,
	},
};

pub type Log<T> = RawLog<<T as Trait>::FinalNum>;
/// The identifier for the `finalnum` inherent.
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"finalnum";

/// Auxiliary trait to extract finalized inherent data.
pub trait FinalizedInherentData<N: Decode> {
	/// Get finalized inherent data.
	fn finalized_number(&self) -> Result<N, RuntimeString>;
}

impl<N: Decode> FinalizedInherentData<N> for InherentData {
	fn finalized_number(&self) -> Result<N, RuntimeString> {
		self.get_data(&INHERENT_IDENTIFIER)
			.and_then(|r| r.ok_or_else(|| "Finalized number inherent data not found".into()))
	}
}

/// Provider for inherent data.
#[cfg(feature = "std")]
pub struct InherentDataProvider<F, N> {
	inner: F,
	_marker: std::marker::PhantomData<N>,
}

#[cfg(feature = "std")]
impl<F, N> InherentDataProvider<F, N> {
	pub fn new(final_oracle: F) -> Self {
		InherentDataProvider { inner: final_oracle, _marker: Default::default() }
	}
}

#[cfg(feature = "std")]
impl<F, N: Encode> inherents::ProvideInherentData for InherentDataProvider<F, N>
	where F: Fn() -> Result<N, RuntimeString>
{
	fn inherent_identifier(&self) -> &'static InherentIdentifier {
		&INHERENT_IDENTIFIER
	}

	fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), RuntimeString> {
		(self.inner)()
			.and_then(|n| inherent_data.put_data(INHERENT_IDENTIFIER, &n))
	}

	fn error_to_string(&self, _error: &[u8]) -> Option<String> {
		Some(format!("no further information"))
	}
}

/// Logs in this module.
#[cfg_attr(feature = "std", derive(Serialize, Debug))]
#[derive(Encode, Decode, PartialEq, Eq, Clone)]
pub enum RawLog<N> {
	/// Block Header digest log for shard info
	FinalizedBlockNumber(N),
}

pub trait Trait: system::Trait {
	type FinalNum: Member + MaybeSerializeDebug + Default + Copy + MaybeDisplay + SimpleArithmetic + Codec;
	type Log: From<Log<Self>> + Into<system::DigestItemOf<Self>>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Timestamp {
		/// Final hint to apply in the block. `None` means "same as parent".
		FinalizedNumber get(finalized_num): Option<T::FinalNum>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		/// Hint that the author of this block thinks the best finalized
		/// block is the given number.
		fn write_finalized_log(origin, hint: T::FinalNum) {
			ensure_inherent(origin)?;

			<Self as Store>::FinalizedNumber::put(hint);
		}

		fn on_finalize() {
			if let Some(final_num) = Self::finalized_num() {
				Self::deposit_log(RawLog::FinalizedBlockNumber(final_num));
			}
		}
	}
}

impl<T: Trait> Module<T> {
	fn deposit_log(log: Log<T>) {
		<system::Module<T>>::deposit_log(<T as Trait>::Log::from(log).into());
	}
}

impl<T: Trait> ProvideInherent for Module<T> {
	type Call = Call<T>;
	type Error = MakeFatalError<()>;
	const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

	fn create_inherent(data: &InherentData) -> Option<Self::Call> {
		let final_num =
			data.finalized_number().expect("Gets and decodes final number inherent data");

		// make hint only when not same as last to avoid bloat.
		Some(Call::write_finalized_log(final_num))
	}

	fn check_inherent(_call: &Self::Call, _data: &InherentData) -> result::Result<(), Self::Error> {
		Ok(())
	}
}
