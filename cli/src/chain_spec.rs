use fg_primitives::{
	MAX_AUTHORITIES_SIZE,
	BLOCK_FINAL_LATENCY,
};

use {
    primitives::{
        sr25519, Pair,
		ed25519::Public as AuthorityId,
		ed25519,
    },
};
use yee_runtime::{
	AccountId, GenesisConfig, ConsensusConfig, TimestampConfig, BalancesConfig,
	IndicesConfig, CrfgConfig,
    PowConfig, ShardingConfig,
	AssetsConfig,
};
use substrate_service;
use yee_primitives::{Address, AddressCodec};
use yee_dev;
use serde::export::PhantomData;
use crate::service::WASM_CODE;

// Note this is the URL for the telemetry server
//const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = substrate_service::ChainSpec<GenesisConfig>;

/// The chain specification option. This is expected to come in from the CLI and
/// is little more than one of a number of alternatives which can easily be converted
/// from a string (`--chain=...`) into a `ChainSpec`.
#[derive(Clone, Debug)]
pub enum Alternative {
	/// Whatever the current runtime is
	Development,
	/// Whatever the current runtime is
	LocalTestnet,
    /// Proof-of-Concept chain with prebuilt runtime.
    POCTestnet,
	/// Witch prebuilt runtime.
	TestNet,
}

fn account_key(s: &str) -> AccountId {
	sr25519::Pair::from_string(&format!("//{}", s), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate AccountId from seed
pub fn get_account_id_from_seed(seed: &str) -> AccountId {
	sr25519::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate AuthorityId from seed
pub fn get_session_key_from_seed(seed: &str) -> AuthorityId {
	ed25519::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, AuthorityId) {
	(
		get_account_id_from_seed(&format!("{}//stash", seed)),
		get_account_id_from_seed(seed),
		get_session_key_from_seed(seed)
	)
}

fn account_addr(s: &str) -> AccountId {

	AccountId::from_address(&Address(s.to_string()))
		.expect("static values are valid; qed").0
}

impl Alternative {
	/// Get an actual chain config from one of the alternatives.
	pub(crate) fn load(self) -> Result<ChainSpec, String> {


		Ok(match self {
			Alternative::Development => ChainSpec::from_genesis(
				"Development",
				"dev",
				|| testnet_genesis(yee_dev::SHARD_CONF.iter().map(|(_, x)| account_addr(x.0)).collect()),
				vec![],
				None,
				None,
				None,
				None
			),
			Alternative::LocalTestnet => ChainSpec::from_genesis(
				"Local Testnet",
				"local_testnet",
				|| testnet_genesis(yee_dev::SHARD_CONF.iter().map(|(_, x)| account_addr(x.0)).collect()),
				vec![],
				None,
				None,
				None,
				None
			),
            Alternative::POCTestnet => ChainSpec::from_genesis(
                "POC Testnet",
                "poc_testnet",
                || poc_testnet_genesis(yee_dev::SHARD_CONF.iter().map(|(_, x)| account_addr(x.0)).collect()),
                vec![],
                None,
                None,
                None,
                None,
            ),
			Alternative::TestNet => ChainSpec::from_genesis(
				"TestNet",
				"testnet",
				|| testnet_genesis(yee_dev::SHARD_CONF.iter().map(|(_,x)| account_addr(x.0)).collect()),
				vec![],
				None,
				None,
				None,
				None,
			),
		})
	}

	pub(crate) fn from(s: &str) -> Option<Self> {
		match s {
			"dev" => Some(Alternative::Development),
            "local" => Some(Alternative::LocalTestnet),
            "poc" => Some(Alternative::POCTestnet),
            "" => Some(Alternative::TestNet),
			_ => None,
		}
	}
}

fn testnet_genesis(endowed_accounts: Vec<AccountId>) -> GenesisConfig {
    let code = WASM_CODE.to_vec();
    testnet_template_genesis(
        endowed_accounts, code,
        primitives::U256::from(0x0000ffff) << 224,
        30,
    )
}

fn poc_testnet_genesis(endowed_accounts: Vec<AccountId>) -> GenesisConfig {
    let code = WASM_CODE.to_vec();
    testnet_template_genesis(
        endowed_accounts, code,
        primitives::U256::from(0x00003fff) << 224,
        30,
    )
}

fn testnet_template_genesis(
    endowed_accounts: Vec<AccountId>,
    code: Vec<u8>,
    genesis_pow_target: primitives::U256,
    target_block_time: u64,
) -> GenesisConfig {
	let block_reward_latency = MAX_AUTHORITIES_SIZE + BLOCK_FINAL_LATENCY + 1;

	GenesisConfig {
		consensus: Some(ConsensusConfig {
			code,
			authorities: vec![],
		}),
		system: None,
		timestamp: Some(TimestampConfig {
			minimum_period: 0, // 10 second block time.
		}),
        pow: Some(PowConfig {
            genesis_pow_target,
            pow_target_adj: 60_u64.into(),
            target_block_time: target_block_time.into(),
	        block_reward: 25600000000,
	        block_reward_latency: block_reward_latency.into(),
        }),
		indices: Some(IndicesConfig {
			ids: endowed_accounts.clone(),
		}),
		balances: Some(BalancesConfig {
			transaction_base_fee: 100,
			transaction_byte_fee: 1,
			existential_deposit: 500,
			transfer_fee: 0,
			creation_fee: 0,
			balances: endowed_accounts.iter().cloned().map(|k|(k, 0)).collect(),
			vesting: vec![],
		}),
		assets: Some(AssetsConfig {
			_genesis_phantom_data: PhantomData,
			next_asset_id: 100,
		}),
        sharding: Some(ShardingConfig {
            genesis_sharding_count: 4,
	        scale_out_observe_blocks: 1,
        }),
		crfg: Some(CrfgConfig {
			authorities: vec![],
		}),
	}
}
