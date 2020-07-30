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
	AssetsConfig, SudoConfig,
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
	/// Main net!!!
	MainNet,
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

pub const BOOTNODES_ROUTER: [&str; 8] = [
	"http://54.92.12.49:6666",
	"http://106.75.164.126:6666",
	"http://106.75.139.215:6666",
	"http://106.75.126.55:6666",
	"http://106.75.36.3:6666",
	"http://23.91.98.161:6666",
	"http://118.193.34.63:6666",
	"http://107.150.124.16:6666",
];

pub const ENDOWED_ACCOUNTS: [(&str, u128); 1] = [
	("yee1m278egdx0fh7tgk8xsk67enjsvl6fqm9ehvjspj27v9dghr7x57qr5l44f", 7_050_275_000_00000000u128),
];

pub const SUDO_ACCOUNTS: [&str; 4] = [
	"yee19q7k8j0hy2rnjtx080zhepdpfzccqp9pwsywz48tclnwse0pdekqgaqw8k",
	"yee1aezky23mll3jhg3gh8l9vkaw9c0mnkxxd29ye27u7anms7rxl52strps30",
	"yee1ns6j0ynkxy2mwc340mf4wlpm72cqqwngh85x94p0ea3tnaq27s3qapctsx",
	"yee136vpzc2ptth3ms2gf29826twhy4tyjzdt7ly0ugm23nnaa7ungpsqrrl88",
];

pub const CHAIN_ID: &'static [u8] = b"mainnet_20200730";

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
				|| testnet_genesis("dev".as_bytes().to_vec(),yee_dev::SHARD_CONF.iter().map(|(_, x)| account_addr(x.0)).collect()),
				vec![],
				None,
				None,
				None,
				None
			),
			Alternative::LocalTestnet => ChainSpec::from_genesis(
				"Local Testnet",
				"local_testnet",
				|| testnet_genesis("local_testnet".as_bytes().to_vec(), yee_dev::SHARD_CONF.iter().map(|(_, x)| account_addr(x.0)).collect()),
				vec![],
				None,
				None,
				None,
				None
			),
            Alternative::POCTestnet => ChainSpec::from_genesis(
                "POC Testnet",
                "poc_testnet",
                || testnet_genesis("poc_testnet".as_bytes().to_vec(), yee_dev::SHARD_CONF.iter().map(|(_, x)| account_addr(x.0)).collect()),
                vec![],
                None,
                None,
                None,
                None,
            ),
			Alternative::TestNet => ChainSpec::from_genesis(
				"TestNet",
				"testnet",
				|| testnet_genesis("testnet".as_bytes().to_vec(), yee_dev::SHARD_CONF.iter().map(|(_,x)| account_addr(x.0)).collect()),
				vec![],
				None,
				None,
				None,
				None,
			),
			Alternative::MainNet => ChainSpec::from_genesis(
				"MainNet",
				"mainnet",
				|| mainnet_genesis(CHAIN_ID.to_vec(), ENDOWED_ACCOUNTS.iter().map(|&(x, value)| (account_addr(x), value)).collect(),
								   SUDO_ACCOUNTS.iter().map(|&x| account_addr(x)).collect()),
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
			"test" => Some(Alternative::TestNet),
            "" => Some(Alternative::MainNet),
			_ => None,
		}
	}
}

fn mainnet_genesis(chain_id: Vec<u8>, endowed_accounts: Vec<(AccountId, u128)>, sudo_accounts: Vec<AccountId>) -> GenesisConfig {
	let code = WASM_CODE.to_vec();
	let block_reward_latency = MAX_AUTHORITIES_SIZE + BLOCK_FINAL_LATENCY + 1;

	GenesisConfig {
		consensus: Some(ConsensusConfig {
			code,
			authorities: vec![],
		}),
		system: None,
		timestamp: Some(TimestampConfig {
			minimum_period: 0,
		}),
		pow: Some(PowConfig {
			chain_id,
			genesis_pow_target: primitives::U256::from(0x000000ffff) << 216,
			pow_target_adj: 60_u64.into(),
			target_block_time: 30_u64.into(),
			block_reward: 25_600_000_000,
			block_reward_latency: block_reward_latency.into(),
		}),
		indices: Some(IndicesConfig {
			ids: endowed_accounts.iter().cloned().map(|item| item.0).collect(),
		}),
		balances: Some(BalancesConfig {
			transaction_base_fee: 10_000_000 ,
			transaction_byte_fee: 100_000,
			existential_deposit: 500,
			transfer_fee: 0,
			creation_fee: 0,
			balances: endowed_accounts.iter().cloned().map(|(k, v)|(k, v)).collect(),
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
		sudo: Some(SudoConfig {
			keys: sudo_accounts,
		}),
	}
}

fn testnet_genesis(chain_id: Vec<u8>, endowed_accounts: Vec<AccountId>) -> GenesisConfig {
    let code = WASM_CODE.to_vec();
    testnet_template_genesis(
		chain_id,
        endowed_accounts, code,
        primitives::U256::from(0x0000ffff) << 224,
        30,
    )
}

fn testnet_template_genesis(
	chain_id: Vec<u8>,
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
			chain_id,
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
		sudo: Some(SudoConfig {
			keys: endowed_accounts,
		}),
	}
}
