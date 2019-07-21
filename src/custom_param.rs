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
    substrate_cli::{impl_augment_clap, AugmentClap},
};
use log::info;
use substrate_service::{FactoryFullConfiguration, ServiceFactory, Configuration};
use crate::error;
use crate::service::{self, NodeConfig};
use yee_runtime::BuildStorage;
use yee_bootnodes_router;
use yee_bootnodes_router::BootnodesRouterConf;

#[derive(Clone, Debug, Default, StructOpt)]
pub struct YeeCliConfig {
    /// Specify miner coin base for block authoring
    #[structopt(long = "coin-base", value_name = "COIN_BASE")]
    pub coin_base: Option<String>,

    /// Specify shard number
    #[structopt(long = "shard-num", value_name = "SHARD_NUM")]
    pub shard_num: u16,

    /// Specify a list of bootnodes-routers
    #[structopt(long = "bootnodes-routers", value_name = "URL")]
    pub bootnodes_routers: Vec<String>,
}

impl_augment_clap!(YeeCliConfig);

fn get_native_bootnodes(bootnodes_router_conf: BootnodesRouterConf, shard_num: u16) -> error::Result<Vec<String>>{

    match bootnodes_router_conf.shards.get(format!("{}",shard_num).as_str()){
        Some(result) => Ok(result.native.clone()),
        None => Err(error::ErrorKind::Msg("Not found shard in bootnodes_router_conf".to_string()).into())
    }

}

pub fn process_custom_args(config: &mut FactoryFullConfiguration<service::Factory>, custom_args: &YeeCliConfig) -> error::Result<()> {

    if let Some(coin_base) = &custom_args.coin_base {
        info!("Coin Base: {}", coin_base);
        config.custom.parse_coin_base(coin_base.to_owned())
            .map_err(|e| format!("Bad coinbase address {:?}", e))?;
    }

    config.custom.shard_num = custom_args.shard_num;
    info!("Shard num: {}", config.custom.shard_num);

    config.custom.set_bootnodes_routers(custom_args.bootnodes_routers.to_owned());
    info!("Bootnodes routers: {:?}", config.custom.bootnodes_routers);

    if config.custom.bootnodes_routers.len() > 0{

        let bootnodes_router_conf = yee_bootnodes_router::client::call(|mut client|{
            let result = client.bootnodes().call().map_err(|e|format!("{:?}", e))?;
            Ok(result)
        }, &config.custom.bootnodes_routers)?;

        info!("Bootnodes router conf: {:?}", bootnodes_router_conf);

        let bootnodes = get_native_bootnodes(bootnodes_router_conf, config.custom.shard_num)?;

        info!("Bootnodes: {:?}", bootnodes);

        config.network.boot_nodes = bootnodes;
    }

    Ok(())
}
