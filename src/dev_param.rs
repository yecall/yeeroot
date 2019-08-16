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
use std::collections::HashMap;
use std::iter;
use std::net::{Ipv4Addr, SocketAddr, IpAddr};
use log::info;
use network::multiaddr::Protocol;

/// shard_num => (coin_base, rpc_port, ws_port, port)
const DEV_SHARD_PARAMS : [(u16, (&str, u16, u16, u16)); 4] = [
    (0, ("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", 9933, 9944, 30333)),
    (1, ("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", 19933, 19944, 31333)),
    (2, ("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", 29933, 29944, 32333)),
    (3, ("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", 39933, 39944, 33333))
];

pub fn process_dev_param<F>(config: &mut FactoryFullConfiguration<F>, custom_args: &mut YeeCliConfig) -> error::Result<()>
    where F: ServiceFactory<Configuration=NodeConfig> {

    let chain_spec_id = config.chain_spec.id();

    if chain_spec_id == "dev" {

        let shard_params_map: HashMap<u16, (&str, u16, u16, u16)> = DEV_SHARD_PARAMS
            .iter().cloned().collect();

        let shard_num = custom_args.shard_num;

        let one = shard_params_map.get(&shard_num);
        let one = one.ok_or(error::ErrorKind::Input("Invalid shard num".to_string().into()))?;

        let coin_base = one.0;
        let rpc_port = one.1;
        let ws_port = one.2;
        let port = one.3;

        custom_args.coin_base = Some(one.0.to_string());

        config.rpc_http = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), rpc_port));
        config.rpc_ws = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), ws_port));
        config.network.listen_addresses = vec![
            iter::once(Protocol::Ip4(Ipv4Addr::new(0, 0, 0, 0)))
                .chain(iter::once(Protocol::Tcp(port)))
                .collect()
        ];

        info!("Dev params: ");
        info!("  coin base: {}", coin_base);
        info!("  rpc port: {}", rpc_port);
        info!("  ws port: {}", ws_port);
        info!("  port: {}", port);
    }

    Ok(())
}