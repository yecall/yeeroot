use {
    primitives::{
        sr25519, Pair,
        crypto::Ss58Codec,
		ed25519::Public as AuthorityId,
		ed25519,
    },
};
use yee_runtime::{
	AccountId, GenesisConfig, ConsensusConfig, TimestampConfig, BalancesConfig,
	IndicesConfig,CrfgConfig,
    PowConfig, ShardingConfig,
};
use hex_literal::{hex, hex_impl};
use substrate_service;

// Note this is the URL for the telemetry server
//const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = substrate_service::ChainSpec<GenesisConfig>;

/// The chain specification option. This is expected to come in from the CLI and
/// is little more than one of a number of alternatives which can easily be converted
/// from a string (`--chain=...`) into a `ChainSpec`.
#[derive(Clone, Debug)]
pub enum Alternative {
	/// Whatever the current runtime is, with just Alice as an auth.
	Development,
	/// Whatever the current runtime is, with simple Alice/Bob auths.
	LocalTestnet,
    /// Proof-of-Concept chain with prebuilt runtime.
    POCTestnet,
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
    <AccountId as Ss58Codec>::from_string(s)
        .expect("static values are valid; qed")
}

impl Alternative {
	/// Get an actual chain config from one of the alternatives.
	pub(crate) fn load(self) -> Result<ChainSpec, String> {
		Ok(match self {
			Alternative::Development => ChainSpec::from_genesis(
				"Development",
				"dev",
				|| testnet_genesis(vec![
					account_key("Alice")
				],
				),
				vec![],
				None,
				None,
				None,
				None
			),
			Alternative::LocalTestnet => ChainSpec::from_genesis(
				"Local Testnet",
				"local_testnet",
				|| testnet_genesis(vec![
					account_key("Alice"),
					account_key("Bob"),
					account_key("Charlie"),
					account_key("Dave"),
					account_key("Eve"),
					account_key("Ferdie"),
				],
				),
				vec![],
				None,
				None,
				None,
				None
			),
            Alternative::POCTestnet => ChainSpec::from_genesis(
                "POC Testnet",
                "poc_testnet",
                || poc_testnet_genesis(vec![
                    account_addr("5FpUCxXVR5KbQLf3qsfwxzdczyU74VeNYw9ba3rdocn23svG"),
                    account_addr("5EtYZwFsQR2Ex1abqYFsmTxpHWytPkphS1LDsrCJ2Gr6b695"),
                    account_addr("5Gn4ZNCiPGjBrPa7W1DHDCj83u6R9FyUChafM7nTpvW7iHEi"),
                    account_addr("5DyvtMHN3G9TvqVp6ZFcmLuJaRjSYibt2Sh5Hb32cNTTHVB9"),
                ]),
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
            "" | "poc" => Some(Alternative::POCTestnet),
			_ => None,
		}
	}
}

fn testnet_genesis(endowed_accounts: Vec<AccountId>) -> GenesisConfig {
    let code = include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/yee_runtime_wasm.compact.wasm").to_vec();
    testnet_template_genesis(
        endowed_accounts, code,
        primitives::U256::from(0x0000ffff) << 224,
        15,
    )
}

fn poc_testnet_genesis(endowed_accounts: Vec<AccountId>) -> GenesisConfig {
    let code = include_bytes!("../prebuilt/yee_runtime/poc_testnet.wasm").to_vec();
    testnet_template_genesis(
        endowed_accounts, code,
        primitives::U256::from(0x00003fff) << 224,
        60,
    )
}

fn testnet_template_genesis(
    endowed_accounts: Vec<AccountId>,
    code: Vec<u8>,
    genesis_difficulty: primitives::U256,
    target_block_time: u64,
) -> GenesisConfig {
	let initial_authorities = vec![
			get_authority_keys_from_seed("Alice"),
		];
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
            genesis_difficulty,
            difficulty_adj: 60_u64.into(),
            target_block_time: target_block_time.into(),
        }),
		indices: Some(IndicesConfig {
			ids: endowed_accounts.clone(),
		}),
		balances: Some(BalancesConfig {
			transaction_base_fee: 1,
			transaction_byte_fee: 0,
			existential_deposit: 500,
			transfer_fee: 0,
			creation_fee: 0,
			balances: endowed_accounts.iter().cloned().map(|k|(k, 1 << 60)).collect(),
			vesting: vec![],
		}),
        sharding: Some(ShardingConfig {
            genesis_sharding_count: 4,
        }),
		crfg: Some(CrfgConfig {
			authorities: initial_authorities.iter().map(|x| (x.2.clone(), 1)).collect(),
		}),
	}
}
