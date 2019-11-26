// Copyright 2017-2019 Parity Technologies (UK) Ltd.
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

//! CRFG Consensus module for runtime.
//!
//! This manages the CRFG authority set ready for the native code.
//! These authorities are only for CRFG finality, not for consensus overall.
//!
//! In the future, it will also handle misbehavior reports, and on-chain
//! finality notifications.
//!
//! For full integration with CRFG, the `CrfgApi` should be implemented.
//! The necessary items are re-exported via the `fg_primitives` crate.

#![cfg_attr(not(feature = "std"), no_std)]

// re-export since this is necessary for `impl_apis` in runtime.
pub use crfg_primitives as fg_primitives;

#[cfg(feature = "std")]
use serde::Serialize;
use rstd::{prelude::*, vec};
use parity_codec as codec;
use codec::{Encode, Decode};
use fg_primitives::ScheduledChange;
use srml_support::{Parameter, decl_event, decl_storage, decl_module};
use srml_support::storage::StorageValue;
use srml_support::storage::unhashed::StorageVec;
use primitives::traits::CurrentHeight;
use substrate_primitives::ed25519;
use primitives::traits::MaybeSerializeDebug;
use ed25519::Public as AuthorityId;

use inherents::{
	RuntimeString, InherentIdentifier, ProvideInherent,
	InherentData, MakeFatalError,
};

mod mock;
mod tests;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"LocalKey";

#[cfg(feature = "std")]
pub struct InherentDataProvider {
	local_key: AuthorityId,
}

#[cfg(feature = "std")]
impl InherentDataProvider {
	pub fn new(local_key: AuthorityId) -> Self {
		Self {
			local_key: local_key,
		}
	}
}

#[cfg(feature = "std")]
impl inherents::ProvideInherentData for InherentDataProvider {
	fn inherent_identifier(&self) -> &'static [u8; 8] {
		&INHERENT_IDENTIFIER
	}

	fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), RuntimeString> {
		inherent_data.put_data(INHERENT_IDENTIFIER, &self.local_key)
	}

	fn error_to_string(&self, error: &[u8]) -> Option<String> {
		RuntimeString::decode(&mut &error[..]).map(Into::into)
	}
}

struct AuthorityStorageVec<S: codec::Codec + Default>(rstd::marker::PhantomData<S>);
impl<S: codec::Codec + Default> StorageVec for AuthorityStorageVec<S> {
	type Item = (S, u64);
	const PREFIX: &'static [u8] = crate::fg_primitives::well_known_keys::AUTHORITY_PREFIX;
}

/// The log type of this crate, projected from module trait type.
pub type Log<T> = RawLog<
	<T as system::Trait>::BlockNumber,
	<T as Trait>::SessionKey,
>;

/// Logs which can be scanned by CRFG for authorities change events.
pub trait CrfgChangeSignal<N> {
	/// Try to cast the log entry as a contained signal.
	fn as_signal(&self) -> Option<ScheduledChange<N>>;
	/// Try to cast the log entry as a contained forced signal.
	fn as_forced_signal(&self) -> Option<(N, ScheduledChange<N>)>;
}

/// A logs in this module.
#[cfg_attr(feature = "std", derive(Serialize, Debug))]
#[derive(Encode, Decode, PartialEq, Eq, Clone)]
pub enum RawLog<N, SessionKey> {
	/// Authorities set change has been signaled. Contains the new set of authorities
	/// and the delay in blocks _to finalize_ before applying.
	AuthoritiesChangeSignal(N, Vec<(SessionKey, u64)>),
	/// A forced authorities set change. Contains in this order: the median last
	/// finalized block when the change was signaled, the delay in blocks _to import_
	/// before applying and the new set of authorities.
	ForcedAuthoritiesChangeSignal(N, N, Vec<(SessionKey, u64)>),
}

impl<N: Clone, SessionKey> RawLog<N, SessionKey> {
	/// Try to cast the log entry as a contained signal.
	pub fn as_signal(&self) -> Option<(N, &[(SessionKey, u64)])> {
		match *self {
			RawLog::AuthoritiesChangeSignal(ref delay, ref signal) => Some((delay.clone(), signal)),
			RawLog::ForcedAuthoritiesChangeSignal(_, _, _) => None,
		}
	}

	/// Try to cast the log entry as a contained forced signal.
	pub fn as_forced_signal(&self) -> Option<(N, N, &[(SessionKey, u64)])> {
		match *self {
			RawLog::ForcedAuthoritiesChangeSignal(ref median, ref delay, ref signal) => Some((median.clone(), delay.clone(), signal)),
			RawLog::AuthoritiesChangeSignal(_, _) => None,
		}
	}
}

impl<N, SessionKey> CrfgChangeSignal<N> for RawLog<N, SessionKey>
	where N: Clone, SessionKey: Clone + Into<AuthorityId>,
{
	fn as_signal(&self) -> Option<ScheduledChange<N>> {
		RawLog::as_signal(self).map(|(delay, next_authorities)| ScheduledChange {
			delay,
			next_authorities: next_authorities.iter()
				.cloned()
				.map(|(k, w)| (k.into(), w))
				.collect(),
		})
	}

	fn as_forced_signal(&self) -> Option<(N, ScheduledChange<N>)> {
		RawLog::as_forced_signal(self).map(|(median, delay, next_authorities)| (median, ScheduledChange {
			delay,
			next_authorities: next_authorities.iter()
				.cloned()
				.map(|(k, w)| (k.into(), w))
				.collect(),
		}))
	}
}

pub trait Trait: system::Trait {
	/// Type for all log entries of this module.
	type Log: From<Log<Self>> + Into<system::DigestItemOf<Self>>;

	/// The session key type used by authorities.
	type SessionKey: Parameter + Default + MaybeSerializeDebug;

	/// The event type of this module.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// A stored pending change.
#[derive(Encode, Decode)]
pub struct StoredPendingChange<N, SessionKey> {
	/// The block number this was scheduled at.
	pub scheduled_at: N,
	/// The delay in blocks until it will be applied.
	pub delay: N,
	/// The next authority set.
	pub next_authorities: Vec<(SessionKey, u64)>,
	/// If defined it means the change was forced and the given block number
	/// indicates the median last finalized block when the change was signaled.
	pub forced: Option<N>,
}

decl_event!(
	pub enum Event<T> where <T as Trait>::SessionKey {
		/// New authority set has been applied.
		NewAuthorities(Vec<(SessionKey, u64)>),
	}
);

decl_storage! {
	trait Store for Module<T: Trait> as CrfgFinality {
		// Pending change: (signaled at, scheduled change).
		PendingChange get(pending_change): Option<StoredPendingChange<T::BlockNumber, T::SessionKey>>;
	}
	add_extra_genesis {
		config(authorities): Vec<(T::SessionKey, u64)>;

		build(|storage: &mut primitives::StorageOverlay, _: &mut primitives::ChildrenStorageOverlay, config: &GenesisConfig<T>| {
			use codec::{Encode, KeyedVec};

			let auth_count = config.authorities.len() as u32;
			config.authorities.iter().enumerate().for_each(|(i, v)| {
				storage.insert((i as u32).to_keyed_vec(
					crate::fg_primitives::well_known_keys::AUTHORITY_PREFIX),
					v.encode()
				);
			});
			storage.insert(
				crate::fg_primitives::well_known_keys::AUTHORITY_COUNT.to_vec(),
				auth_count.encode(),
			);
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

        fn update_authorities(origin, info: <T as Trait>::SessionKey){
			use primitives::traits::{Zero, As};

			let mut authorities = <Module<T>>::crfg_authorities();
			while authorities.len() >= crate::fg_primitives::MAX_AUTHORITIES_SIZE.as_() {
				authorities.remove(0);
			}
			authorities.push((info, 1));

			let scheduled_at = system::ChainContext::<T>::default().current_height();
			<PendingChange<T>>::put(StoredPendingChange {
				delay: T::BlockNumber::sa(0),
				scheduled_at,
				next_authorities: authorities,
				forced: None,
			});
        }

		fn on_finalize(block_number: T::BlockNumber) {
			if let Some(pending_change) = <PendingChange<T>>::get() {
				if block_number == pending_change.scheduled_at {
					if let Some(median) = pending_change.forced {
						Self::deposit_log(RawLog::ForcedAuthoritiesChangeSignal(
							median,
							pending_change.delay,
							pending_change.next_authorities.clone(),
						));
					} else {
						Self::deposit_log(RawLog::AuthoritiesChangeSignal(
							pending_change.delay,
							pending_change.next_authorities.clone(),
						));
					}
				}

				if block_number == pending_change.scheduled_at + pending_change.delay {
					Self::deposit_event(
						RawEvent::NewAuthorities(pending_change.next_authorities.clone())
					);
					<AuthorityStorageVec<T::SessionKey>>::set_items(pending_change.next_authorities);
					<PendingChange<T>>::kill();
				}
			}
		}
	}
}

impl<T: Trait> Module<T> {
	/// Get the current set of authorities, along with their respective weights.
	pub fn crfg_authorities() -> Vec<(T::SessionKey, u64)> {
		<AuthorityStorageVec<T::SessionKey>>::items()
	}

	/// Deposit one of this module's logs.
	fn deposit_log(log: Log<T>) {
		<system::Module<T>>::deposit_log(<T as Trait>::Log::from(log).into());
	}
}

impl<T: Trait> Module<T> where AuthorityId: core::convert::From<<T as Trait>::SessionKey> {
	/// See if the digest contains any standard scheduled change.
	pub fn scrape_digest_change(log: &Log<T>)
		-> Option<ScheduledChange<T::BlockNumber>>
	{
		<Log<T> as CrfgChangeSignal<T::BlockNumber>>::as_signal(log)
	}

	/// See if the digest contains any forced scheduled change.
	pub fn scrape_digest_forced_change(log: &Log<T>)
		-> Option<(T::BlockNumber, ScheduledChange<T::BlockNumber>)>
	{
		<Log<T> as CrfgChangeSignal<T::BlockNumber>>::as_forced_signal(log)
	}
}

/// Helper for authorities being synchronized with the general session authorities.
///
/// This is not the only way to manage an authority set for CRFG, but it is
/// a convenient one. When this is used, no other mechanism for altering authority
/// sets should be.
pub struct SyncedAuthorities<T>(::rstd::marker::PhantomData<T>);

// FIXME: remove when https://github.com/rust-lang/rust/issues/26925 is fixed
impl<T> Default for SyncedAuthorities<T> {
	fn default() -> Self {
		SyncedAuthorities(::rstd::marker::PhantomData)
	}
}

impl<T: Trait> ProvideInherent for Module<T> {
	type Call = Call<T>;
	type Error = MakeFatalError<RuntimeString>;
	const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

	fn create_inherent(data: &InherentData) -> Option<Self::Call> {
		let data = extract_inherent_data(data)
			.expect("Crfg inherent data must exist");

		Some(Call::update_authorities(data))
	}

	fn check_inherent(_: &Self::Call, _: &InherentData) -> Result<(), Self::Error> {
		Ok(())
	}
}

fn extract_inherent_data<SessionKey>(data: &InherentData) -> Result<SessionKey, RuntimeString>
	where SessionKey: Decode
{
	data.get_data::<SessionKey>(&INHERENT_IDENTIFIER)
		.map_err(|_| RuntimeString::from("Invalid authorities inherent data encoding."))?
		.ok_or_else(|| "Authorities inherent data is not provided.".into())
}
