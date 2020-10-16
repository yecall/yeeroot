use std::io;
use std::path::{Path, PathBuf};

use app_dirs::{AppDataType, AppInfo};
use parity_codec::alloc::collections::HashMap;
use structopt::StructOpt;
use substrate_cli::SharedParams;
use substrate_cli::VersionInfo;
use substrate_service::{ChainSpec, new_light_client, new_full_client, Configuration, FactoryFullConfiguration, FactoryGenesis, RuntimeGenesis, ServiceFactory};
use log::info;
use runtime_primitives::traits::As;
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
        if shard == cli.shard_num {
            config.database_path = db_path(&base_path, spec_id.as_str(), true, shard).to_string_lossy().into();
            let client = new_full_client::<F>(&config)?;
            let best = client.revert(As::sa(number))?;
            info!("Reverted shard: {}. Best: #{}", shard, best);
        } else {
            config.database_path = db_path(&base_path, spec_id.as_str(), false, shard).to_string_lossy().into();
            let client = new_light_client::<F>(&config)?;
            let best = client.revert(As::sa(number))?;
            info!("Reverted shard: {}. Best: #{}", shard, best);
        }
    }

    Ok(())
}

fn create_config<F, S>(
    spec_factory: S, cli: &SharedParams, version: &VersionInfo,
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

fn revert_full() -> io::Result<()> {
    Ok(())
}

fn revert_light() -> io::Result<()> {
    Ok(())
}
