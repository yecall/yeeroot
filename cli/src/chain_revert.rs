use std::io;
use std::path::{Path, PathBuf};

use app_dirs::{AppDataType, AppInfo};
use log::info;
use parity_codec::{Decode, Encode};
use parity_codec::alloc::collections::HashMap;
use primitives::H256;
use runtime_primitives::generic::{BlockId, DigestItem};
use runtime_primitives::traits::{Block as BlockT, Digest, DigestItemFor, Header as HeaderT, NumberFor};
use runtime_primitives::traits::As;
use serde_json::Value;
use structopt::StructOpt;
use substrate_cli::SharedParams;
use substrate_cli::VersionInfo;
use substrate_client::backend::AuxStore;
use substrate_service::{ChainSpec, Configuration, FactoryFullConfiguration, FactoryGenesis, new_full_client, new_light_client, RuntimeGenesis, ServiceFactory};

use crfg::{authorities, aux_schema, CrfgChangeDigestItem, ScheduledChange};
use finality_grandpa::round::State;
use yee_foreign_network::message::generic::Message::Status;

use crate::chain_spec;
use crate::error;

/// The `revert` command used revert the chain to a previos state.
#[derive(Debug, StructOpt, Clone)]
pub struct RevertCmd {
    /// Specify shard number
    #[structopt(long = "shard-num", value_name = "SHARD_NUM")]
    pub shard_num: u16,

    /// Number of blocks to revert.
    #[structopt(long = "target", value_name = "TARGET")]
    pub target: String,

    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub shared_params: SharedParams,
}

impl substrate_cli::GetLogFilter for RevertCmd {
    fn get_log_filter(&self) -> Option<String> {
        None
    }
}

pub fn revert_chain<F, S>(cli: RevertCmd, version: VersionInfo, spec_factory: S) -> error::Result<()> where
    F: ServiceFactory,
    S: FnOnce(&str) -> Result<Option<ChainSpec<FactoryGenesis<F>>>, String>,
    <<<<F as ServiceFactory>::Block as BlockT>::Header as HeaderT>::Digest as Digest>::Item: CrfgChangeDigestItem<<<<F as ServiceFactory>::Block as BlockT>::Header as HeaderT>::Number>,
{
    let target = match serde_json::from_str(cli.target.as_str()) {
        Ok(v) => {
            let r: HashMap<u16, u64> = v;
            r
        }
        _ => {
            panic!("Args: target format error.");
        }
    };
    let (spec_id, mut config) = create_config::<F, _>(spec_factory, &cli.shared_params, &version);

    let base_path = base_path(&cli.shared_params, &version);
    for (shard, number) in target {
        let empty: Vec<u8> = vec![];
        let empty = empty.encode();
        if shard == cli.shard_num {
            config.database_path = db_path(&base_path, spec_id.as_str(), true, shard).to_string_lossy().into();
            let client = new_full_client::<F>(&config)?;
            let best = client.revert(As::sa(number))?;
            let header = client.header(&BlockId::Number(best))?.expect("can't get header");
            let hash = header.hash();
            let authorities = match get_authorities::<F::Block>(header, number) {
                Ok(v) => v,
                Err(e) => panic!("{:?}", e)
            };
            client.insert_aux(&[(aux_schema::AUTHORITY_SET_KEY, authorities.encode().as_slice())], &[])?;

            let state = State {
                prevote_ghost: Some((hash, number)),
                finalized: Some((hash, number)),
                estimate: Some((hash, number)),
                completable: true,
            };
            let set_state = aux_schema::VoterSetState::Live(0u64, state);
            client.insert_aux(&[(aux_schema::SET_STATE_KEY, set_state.encode().as_slice())], &[])?;
            client.insert_aux(&[(aux_schema::CONSENSUS_CHANGES_KEY, empty.as_slice())], &[])?;
            client.insert_aux(&[(aux_schema::PENDING_SKIP_KEY, empty.as_slice())], &[])?;
            info!("Reverted shard: {}. Best: #{}", shard, best);
        } else {
            config.database_path = db_path(&base_path, spec_id.as_str(), false, shard).to_string_lossy().into();
            let client = new_light_client::<F>(&config)?;
            let best = client.revert(As::sa(number))?;
            let header = client.header(&BlockId::Number(best))?.expect("can't get header");
            let hash = header.hash();
            let authorities = match get_authorities::<F::Block>(header, number) {
                Ok(v) => v,
                Err(e) => panic!("{:?}", e)
            };
            client.insert_aux(&[(aux_schema::AUTHORITY_SET_KEY, authorities.encode().as_slice())], &[])?;
            let state = State {
                prevote_ghost: Some((hash, number)),
                finalized: Some((hash, number)),
                estimate: Some((hash, number)),
                completable: true,
            };
            let set_state = aux_schema::VoterSetState::Live(0u64, state);
            client.insert_aux(&[(aux_schema::SET_STATE_KEY, set_state.encode().as_slice())], &[])?;
            client.insert_aux(&[(aux_schema::CONSENSUS_CHANGES_KEY, empty.as_slice())], &[])?;
            client.insert_aux(&[(aux_schema::PENDING_SKIP_KEY, empty.as_slice())], &[])?;
            info!("Reverted shard: {}. Best: #{}", shard, best);
        }
    }

    Ok(())
}

fn create_config<F, S>(
    spec_factory: S, cli: &SharedParams, _version: &VersionInfo,
) -> (String, FactoryFullConfiguration<F>)
    where
        F: ServiceFactory,
        S: FnOnce(&str) -> Result<Option<ChainSpec<FactoryGenesis<F>>>, String>,
{
    let spec = match load_spec(cli, spec_factory) {
        Ok(v) => v,
        Err(e) => {
            panic!("{:?}", e);
        }
    };

    (spec.id().to_string(), Configuration::default_with_spec(spec.clone()))
}

fn load_spec<F, G>(cli: &SharedParams, factory: F) -> error::Result<ChainSpec<G>>
    where G: RuntimeGenesis, F: FnOnce(&str) -> Result<Option<ChainSpec<G>>, String>,
{
    let chain_key = get_chain_key(cli);
    let spec = match factory(&chain_key)? {
        Some(spec) => spec,
        None => ChainSpec::from_json_file(PathBuf::from(chain_key))?
    };
    Ok(spec)
}

fn base_path(cli: &SharedParams, version: &VersionInfo) -> PathBuf {
    cli.base_path.clone()
        .unwrap_or_else(||
            app_dirs::get_app_root(
                AppDataType::UserData,
                &AppInfo {
                    name: version.executable_name,
                    author: version.author,
                },
            ).expect("app directories exist on all supported platforms; qed")
        )
}

fn get_chain_key(cli: &SharedParams) -> String {
    match cli.chain {
        Some(ref chain) => chain.clone(),
        None => if cli.dev { "dev".into() } else { "".into() }
    }
}

fn db_path(base_path: &Path, chain_id: &str, is_full: bool, shard_num: u16) -> PathBuf {
    let mut path = base_path.to_owned();
    path.push("chains");
    path.push(chain_id);
    if is_full {
        path.push("db");
    } else {
        path.push(format!("db-{}", shard_num));
    }
    path
}

type AuthorityId = [u8; 32];
type Hash = [u8; 32];

fn get_authorities<B: BlockT>(header: B::Header, number: u64) -> Result<authorities::AuthoritySet<Hash, u64>, String>
where
        DigestItemFor<B>: CrfgChangeDigestItem<NumberFor<B>>,
        <<<B as BlockT>::Header as HeaderT>::Digest as Digest>::Item: CrfgChangeDigestItem<<<B as BlockT>::Header as HeaderT>::Number>,
{
    let change: Option<ScheduledChange<<B::Header as HeaderT>::Number>> = header.digest().logs().iter()
        .filter_map(CrfgChangeDigestItem::as_change)
        .next();

    match change {
        Some(v) => {
            let authority_set = aux_schema::V0AuthoritySet::<[u8; 32], u64> {
                current_authorities: v.next_authorities,
                pending_changes: Vec::new(),
                set_id: number,
            };
            let new_set: authorities::AuthoritySet<Hash, u64> = authority_set.into();
            Ok(new_set)
        }
        None => Err(String::from("can't get ScheduledChange"))
    }
}