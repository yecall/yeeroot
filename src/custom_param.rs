// Copyright (C) 2019 Yee Foundation.
//
// This file is part of YeeChain.
//
// YeeChain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// YeeChain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with YeeChain.  If not, see <https://www.gnu.org/licenses/>.

use {
    structopt::StructOpt,
    substrate_cli::{impl_augment_clap},
};
use log::{info, warn};
use substrate_service::{FactoryFullConfiguration, ServiceFactory, config::Roles};
use crate::error;
use crate::service::{NodeConfig};
use yee_bootnodes_router;
use yee_bootnodes_router::BootnodesRouterConf;
use yee_runtime::AccountId;
use yee_primitives::{AddressCodec, Address, Hrp};

#[derive(Clone, Debug, Default, StructOpt)]
pub struct YeeCliConfig {
    /// Specify miner coinbase for block authoring
    #[structopt(long = "coinbase", value_name = "COINBASE")]
    pub coinbase: Option<String>,

    /// Specify shard number
    #[structopt(long = "shard-num", value_name = "SHARD_NUM")]
    pub shard_num: u16,

    /// Specify a list of bootnodes-routers
    #[structopt(long = "bootnodes-routers", value_name = "URL")]
    pub bootnodes_routers: Vec<String>,

    /// Specify foreign p2p protocol TCP port
    #[structopt(long = "foreign-port", value_name = "PORT")]
    pub foreign_port: Option<u16>,

    /// Whether use dev params or not
    #[structopt(long = "dev-params")]
    pub dev_params: bool,

    /// Whether mine
    #[structopt(long = "mine")]
    pub mine: bool,
}

impl_augment_clap!(YeeCliConfig);

pub fn process_custom_args<F>(config: &mut FactoryFullConfiguration<F>, custom_args: &YeeCliConfig) -> error::Result<()>
where F: ServiceFactory<Configuration=NodeConfig<F>>{

    config.custom.hrp = get_hrp(config.chain_spec.id());

    config.custom.shard_num = custom_args.shard_num;

    if config.roles == Roles::AUTHORITY{
        let coinbase = custom_args.coinbase.clone().ok_or(error::ErrorKind::Input("Coinbase not found".to_string()))?;
        let (coinbase, hrp) = parse_coinbase(coinbase).map_err(|e| format!("Invalid coinbase: {:?}", e))?;

        if config.custom.hrp != hrp{
            return Err(error::ErrorKind::Input("Invalid coinbase hrp".to_string()).into());
        }

        config.custom.coinbase = coinbase;
    }

    let bootnodes_routers = custom_args.bootnodes_routers.clone();

    if bootnodes_routers.len() > 0{

        match get_bootnodes_router_conf(&bootnodes_routers){
            Ok(bootnodes_router_conf) => {

                match get_native_bootnodes(&bootnodes_router_conf, config.custom.shard_num){
                    Ok(bootnodes) => {
                        config.network.boot_nodes = bootnodes;
                    },
                    Err(_e) => {
                        warn!("Failed to get bootnodes: {:?}", bootnodes_routers);
                    }
                }

                config.custom.bootnodes_router_conf = Some(bootnodes_router_conf);
            },
            Err(_) => {
                warn!("Failed to get bootnodes router conf: {:?}", bootnodes_routers);
            }
        }
    }

    config.custom.foreign_port = custom_args.foreign_port;
    config.custom.mine = custom_args.mine;

    info!("Custom params: ");
    info!("  coinbase: {}", config.custom.coinbase.to_address(config.custom.hrp.clone()).expect("qed"));
    info!("  shard num: {}", config.custom.shard_num);
    info!("  bootnodes: {:?}", config.network.boot_nodes);
    info!("  foreign port: {:?}", config.custom.foreign_port);
    info!("  bootnodes router conf: {:?}", config.custom.bootnodes_router_conf);
    info!("  mine: {:?}", config.custom.mine);

    Ok(())
}

fn get_bootnodes_router_conf(bootnodes_routers :&Vec<String>) -> error::Result<BootnodesRouterConf>{

    yee_bootnodes_router::client::call(|mut client|{
        let result = client.bootnodes().call().map_err(|e|format!("{:?}", e))?;
        Ok(result)
    }, bootnodes_routers).map_err(|e|format!("{:?}", e).into())

}

fn get_native_bootnodes(bootnodes_router_conf: &BootnodesRouterConf, shard_num: u16) -> error::Result<Vec<String>>{

    match bootnodes_router_conf.shards.get(format!("{}",shard_num).as_str()){
        Some(result) => Ok(result.native.clone()),
        None => Err(error::ErrorKind::Msg("Not found shard in bootnodes_router_conf".to_string()).into())
    }
}

fn parse_coinbase(input: String) -> error::Result<(AccountId, Hrp)> {

    let address = Address(input);
    let (coinbase, hrp) = AccountId::from_address(&address).map_err(|e| format!("{:?}", e))?;

    Ok((coinbase, hrp))
}

fn get_hrp(chain_spec_id: &str) -> Hrp{

    match chain_spec_id{
        "main" => Hrp::MAINNET,
        _ => Hrp::TESTNET,
    }
}
