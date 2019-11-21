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

//! setup default params for dev mode

use crate::error;
use substrate_service::{FactoryFullConfiguration, ServiceFactory};
use crate::service::NodeConfig;
use crate::custom_param::YeeCliConfig;
use std::iter;
use std::net::{Ipv4Addr, SocketAddr, IpAddr};
use log::info;
use network::multiaddr::Protocol;

pub fn process_dev_param<F, C>(config: &mut FactoryFullConfiguration<F>, custom_args: &mut YeeCliConfig) -> error::Result<()>
    where F: ServiceFactory<Configuration=NodeConfig<F>> {

    let chain_spec_id = config.chain_spec.id();

    if chain_spec_id == "dev" && custom_args.dev_params {

        let shard_num = custom_args.shard_num;

        let run_params = yee_dev::get_run_params(shard_num).map_err(|e| format!("{:?}", e))?;

        info!("Dev params: ");
        info!("  coinbase: {}", run_params.coinbase);
        info!("  rpc port: {}", run_params.rpc_port);
        info!("  ws port: {}", run_params.ws_port);
        info!("  port: {}", run_params.port);
        info!("  node key: {}", yee_dev::get_peer_id(&run_params.node_key_config));
        info!("  foreign port: {}", run_params.foreign_port);
        info!("  bootnodes routers: {:?}", run_params.bootnodes_routers);

        custom_args.coinbase = Some(run_params.coinbase);
        custom_args.foreign_port = Some(run_params.foreign_port);
        custom_args.bootnodes_routers = run_params.bootnodes_routers;

        config.rpc_http = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), run_params.rpc_port));
        config.rpc_ws = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), run_params.ws_port));
        config.network.listen_addresses = vec![
            iter::once(Protocol::Ip4(Ipv4Addr::new(0, 0, 0, 0)))
                .chain(iter::once(Protocol::Tcp(run_params.port)))
                .collect()
        ];

        config.network.node_key = run_params.node_key_config;

    }

    Ok(())
}
