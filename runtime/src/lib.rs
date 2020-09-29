//! The Substrate Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit="256"]

#[cfg(feature = "std")]
use serde::{Serialize, Deserialize};
use parity_codec::{Encode, Decode};
use rstd::prelude::*;
#[cfg(feature = "std")]
use primitives::bytes;
use primitives::{ed25519, sr25519, OpaqueMetadata};
use runtime_primitives::{
	ApplyResult, transaction_validity::TransactionValidity, generic, create_runtime_str,
	traits::{self, NumberFor, BlakeTwo256, Block as BlockT, DigestFor, StaticLookup, Verify, As}
};
use finality_tracker;
use crfg::fg_primitives::{self, ScheduledChange};
use client::{
	block_builder::api::{CheckInherentsResult, InherentData, self as block_builder_api},
	runtime_api, impl_runtime_apis
};
use sharding_primitives::ShardingInfo;
use version::RuntimeVersion;
#[cfg(feature = "std")]
use version::NativeVersion;

pub use {
    consensus_pow::PowTarget,
};

// A few exports that help ease life for downstream crates.
#[cfg(any(feature = "std", test))]
pub use runtime_primitives::BuildStorage;
pub use consensus::Call as ConsensusCall;
pub use timestamp::Call as TimestampCall;
pub use relay::Call as RelayCall;
pub use balances::Call as BalancesCall;
pub use assets::Call as AssetsCall;
pub use runtime_primitives::{Permill, Perbill};
pub use timestamp::BlockPeriod;
pub use support::{StorageValue, construct_runtime};

/// The type that is used for identifying authorities.
pub type AuthorityId = <AuthoritySignature as Verify>::Signer;

/// The type used by authorities to prove their ID.
pub type AuthoritySignature = ed25519::Signature;

/// Alias to pubkey that identifies an account on the chain.
pub type AccountId = <AccountSignature as Verify>::Signer;

/// The type used by authorities to prove their ID.
pub type AccountSignature = sr25519::Signature;

/// A hash of some data used by the chain.
pub type Hash = primitives::H256;

/// Index of a block number in the chain.
pub type BlockNumber = u64;

/// Index of an account's extrinsic in the chain.
pub type Nonce = u64;

/// Yee module
mod yee;

#[cfg(feature = "std")]
pub use sharding::GenesisConfig as ShardingGenesisConfig;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core datastructures.
pub mod opaque {
	use super::*;

	/// Opaque, encoded, unchecked extrinsic.
	#[derive(PartialEq, Eq, Clone, Default, Encode, Decode)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub struct UncheckedExtrinsic(#[cfg_attr(feature = "std", serde(with="bytes"))] pub Vec<u8>);
	#[cfg(feature = "std")]
	impl std::fmt::Debug for UncheckedExtrinsic {
		fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
			write!(fmt, "{}", primitives::hexdisplay::HexDisplay::from(&self.0))
		}
	}
	impl traits::Extrinsic for UncheckedExtrinsic {
		fn is_signed(&self) -> Option<bool> {
			None
		}
	}
	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256, generic::DigestItem<Hash, AuthorityId, AuthoritySignature>>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
	/// Opaque session key type.
	pub type SessionKey = AuthorityId;
}

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("yee"),
	impl_name: create_runtime_str!("yee-rs"),
	authoring_version: 4,
	spec_version: 5,
	impl_version: 5,
	apis: RUNTIME_API_VERSIONS,
};

/// The version infromation used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

impl system::Trait for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = Indices;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Nonce;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header digest type.
	type Digest = generic::Digest<Log>;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256, Log>;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous log type.
	type Log = Log;
	/// The ubiquitous origin type.
	type Origin = Origin;
}

impl assets::Trait for Runtime {
	type Event = Event;

	type Balance = u128;

	type Sharding = sharding::Module<Runtime>;
}

impl relay::Trait for Runtime {
	type Runtime = Runtime;
}

impl storage::Trait for Runtime {
	type Event = Event;
}

impl pow::Trait for Runtime {
    /// Type used for POW target
    type PowTarget = PowTarget;
	/// Type used for reward
	type Currency = balances::Module<Self>;

	type Reward = ();

	type Event = Event;

	type Sharding = sharding::Module<Runtime>;

}

impl consensus::Trait for Runtime {
	/// The identifier we use to refer to authorities.
	type SessionKey = AuthorityId;
	// The aura module handles offline-reports internally
	// rather than using an explicit report system.
	type InherentOfflineReport = ();
	/// The ubiquitous log type.
	type Log = Log;
}

impl indices::Trait for Runtime {
	/// The type for recording indexing into the account enumeration. If this ever overflows, there
	/// will be problems!
	type AccountIndex = u32;
	/// Use the standard means of resolving an index hint from an id.
	type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
	/// Determine whether an account is dead.
	type IsDeadAccount = Balances;
	/// The uniquitous event type.
	type Event = Event;
}

impl timestamp::Trait for Runtime {
	/// A timestamp: seconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = ();
}

impl balances::Trait for Runtime {
	/// The type for recording an account's balance.
	type Balance = u128;
	/// What to do if an account's free balance gets zeroed.
	type OnFreeBalanceZero = ();
	/// What to do if a new account is created.
	type OnNewAccount = Indices;
	/// The uniquitous event type.
	type Event = Event;

	type TransactionPayment = ();
	type DustRemoval = ();
	type TransferPayment = ();

    type Sharding = sharding::Module<Runtime>;
	type OnFeeWithdrawn = pow::Module<Runtime>;
}

impl finality_tracker::Trait for Runtime {
	type Log = Log;
	type OnFinalizationStalled = crfg::Module<Runtime>;
}

impl crfg::Trait for Runtime {
	type SessionKey = AuthorityId;
	type Log = Log;
	type Event = Event;
}

impl sharding::Trait for Runtime {
    type ShardNum = u16;
    type Log = Log;
}

impl sudo::Trait for Runtime {
	type Event = Event;
	type Proposal = Call;
}

/// DON'T CHANGE THE SORT
construct_runtime!(
	pub enum Runtime with Log(InternalLog: DigestItem<Hash, AuthorityId, AuthoritySignature>) where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: system::{default, Log(ChangesTrieRoot)},
		Timestamp: timestamp::{Module, Call, Storage, Config<T>, Inherent},
		Consensus: consensus::{Module, Call, Storage, Config<T>, Log(AuthoritiesChange), Inherent},
		Pow: pow::{Module, Call, Storage, Config<T>, Inherent, Event<T>},
		Indices: indices,
		Balances: balances,
		Sharding: sharding::{Module, Call, Storage, Config<T>, Log(), Inherent},
		Crfg: crfg::{Module, Call, Storage, Config<T>, Log(), Event<T>, Inherent},
		FinalityTracker: finality_tracker::{Module, Call, Log(), Inherent},
		Assets: assets::{Module, Call, Storage, Config<T>, Event<T>},
		Relay: relay::{Module, Call},
		Storage: storage::{Module, Call, Event<T>},
		Sudo: sudo,
	}
);

/// The type used as a helper for interpreting the sender of transactions.
type Context = ChainContext<Runtime>;
/// The address format for describing accounts.
type Address = <Indices as StaticLookup>::Source;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256, Log>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedMortalCompactExtrinsic<Address, Nonce, Call, AccountSignature>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Nonce, Call>;
/// Executive: handles dispatch to the various modules.
pub type Executive = executive::Executive<Runtime, Block, Context, Balances, AllModules>;

#[cfg(feature = "std")]
impl Clone for GenesisConfig {
    fn clone(&self) -> Self {
        unreachable!()
    }
}

// Implement our runtime API endpoints. This is just a bunch of proxying.
impl_runtime_apis! {
	impl runtime_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block, extra: Option<Vec<u8>>) -> Vec<u8> {
			Executive::execute_block(block, extra)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}

		fn authorities() -> Vec<AuthorityId> {
			panic!("Deprecated, please use `AuthoritiesApi`.")
		}
	}

	impl runtime_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl block_builder_api::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: InherentData) -> CheckInherentsResult {
			data.check_extrinsics(&block)
		}

		fn random_seed() -> <Block as BlockT>::Hash {
			System::random_seed()
		}
	}

	impl runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(tx: <Block as BlockT>::Extrinsic) -> TransactionValidity {
			let count = Sharding::current_shard_info()
                .map_or_else(|| Sharding::genesis_sharding_count(), |info| info.count.into());
			Executive::validate_transaction(tx, count as u16)
		}
	}

	impl consensus_pow::YeePOWApi<Block> for Runtime {
	    fn genesis_pow_target() -> PowTarget {
	        Pow::genesis_pow_target()
	    }

        fn pow_target_adj() -> NumberFor<Block> {
            Pow::pow_target_adj()
        }

        fn target_block_time() -> u64 {
            Pow::target_block_time()
        }
	}

	impl offchain_primitives::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(n: NumberFor<Block>) {
			Executive::offchain_worker(n)
		}
	}

	impl consensus_authorities::AuthoritiesApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityId> {
			Consensus::authorities()
		}
	}

	impl fg_primitives::CrfgApi<Block> for Runtime {
		fn crfg_pending_change(digest: &DigestFor<Block>)
			-> Option<ScheduledChange<NumberFor<Block>>>
		{
			for log in digest.logs.iter().filter_map(|l| match l {
				Log(InternalLog::crfg(crfg_signal)) => Some(crfg_signal),
				_ => None
			}) {
				if let Some(change) = Crfg::scrape_digest_change(log) {
					return Some(change);
				}
			}
			None
		}

		fn crfg_forced_change(digest: &DigestFor<Block>)
			-> Option<(NumberFor<Block>, ScheduledChange<NumberFor<Block>>)>
		{
			for log in digest.logs.iter().filter_map(|l| match l {
				Log(InternalLog::crfg(crfg_signal)) => Some(crfg_signal),
				_ => None
			}) {
				if let Some(change) = Crfg::scrape_digest_forced_change(log) {
					return Some(change);
				}
			}
			None
		}

		fn crfg_authorities() -> Vec<(AuthorityId, u64)> {
			Crfg::crfg_authorities()
		}
	}

	impl sharding_primitives::ShardingAPI<Block> for Runtime {
        fn get_genesis_shard_count() -> u16 {
            Sharding::genesis_sharding_count()
        }

        fn get_curr_shard() -> Option<u16> {
            Sharding::current_shard_info()
                .map(|info| info.num.into())
        }

        fn get_shard_count() -> u16 {
            Sharding::current_shard_info()
                .map_or_else(|| Sharding::genesis_sharding_count(), |info| info.count.into())
        }

		fn get_scale_out_observe_blocks() -> NumberFor<Block> {
			Sharding::scale_out_observe_blocks()
		}
	}
}


pub struct ChainContext<T>(::rstd::marker::PhantomData<T>);
impl<T> Default for ChainContext<T> {
	fn default() -> Self {
		ChainContext(::rstd::marker::PhantomData)
	}
}

impl<T: system::Trait> traits::Lookup for ChainContext<T> {
	type Source = <T::Lookup as StaticLookup>::Source;
	type Target = <T::Lookup as StaticLookup>::Target;
	fn lookup(&self, s: Self::Source) -> rstd::result::Result<Self::Target, &'static str> {
		<T::Lookup as StaticLookup>::lookup(s)
	}
}

impl<T: system::Trait> traits::CurrentHeight for ChainContext<T> {
	type BlockNumber = T::BlockNumber;
	fn current_height(&self) -> Self::BlockNumber {
		<system::Module<T>>::block_number()
	}
}

impl<T: system::Trait> traits::BlockNumberToHash for ChainContext<T> {
	type BlockNumber = T::BlockNumber;
	type Hash = T::Hash;
	fn block_number_to_hash(&self, n: Self::BlockNumber) -> Option<Self::Hash> {
		Some(<system::Module<T>>::block_hash(n))
	}
}

impl<T: system::Trait + sharding::Trait + sudo::Trait> traits::CheckSender for ChainContext<T> {
	type Sender = <T as system::Trait>::AccountId;
	fn check_sender(&self, sender: &Self::Sender) -> bool {
		/// check shard
		let (shard_num, shard_count) : (Option<u16>, u16) =
			(<sharding::Module<T>>::get_curr_shard().map(As::as_).map(|x|x as u16), <sharding::Module<T>>::get_shard_count().as_() as u16);
		let sender_shard_num = sharding_primitives::utils::shard_num_for(sender, shard_count);
		if sender_shard_num.is_some() && sender_shard_num == shard_num {
			return true;
		}
		false
	}
}
