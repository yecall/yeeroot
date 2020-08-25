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

pub mod params;
pub mod config;
pub mod error;

use substrate_cli::VersionInfo;
use crate::params::SwitchCommandCmd;
use log::info;
use futures::future::Future;
use std::net::SocketAddr;
use yee_switch_rpc::author::Author;
use yee_switch_rpc::state::State;
use yee_switch_rpc::system::System;
use yee_switch_rpc::chain::Chain;
use yee_switch_rpc::pow::Pow;
use crate::config::get_config;
use crate::params::DEFAULT_RPC_PORT;
use crate::params::DEFAULT_WS_PORT;
use tokio::runtime::Runtime;
use yee_mining2::work_manager::WorkManagerConfig;

pub const TARGET: &str = "switch";

pub fn run(cmd: SwitchCommandCmd, version: VersionInfo) -> error::Result<()> {
    let config = get_config(&cmd, &version)?;

    let rpc_config: yee_primitives::Config = config.into();


    let rpc_interface: &str = if cmd.rpc_external { "0.0.0.0" } else { "127.0.0.1" };

    let ws_interface: &str = if cmd.ws_external { "0.0.0.0" } else { "127.0.0.1" };

    let rpc_address_http = parse_address(&format!("{}:{}", rpc_interface, DEFAULT_RPC_PORT), cmd.rpc_port)?;

    let rpc_address_ws = parse_address(&format!("{}:{}", ws_interface, DEFAULT_WS_PORT), cmd.ws_port)?;

    let (signal, exit) = exit_future::signal();

    let work_manger = if cmd.enable_work_manager || cmd.mine {
        let work_manager_config = WorkManagerConfig {
            job_refresh_interval: cmd.job_refresh_interval,
            job_cache_size: cmd.job_cache_size,
        };
        Some(yee_mining2::start_work_manager(rpc_config.clone(), work_manager_config)?)
    } else {
        None
    };

    if cmd.mine {
        let work_manager = work_manger.clone().expect("qed");
        yee_mining2::start_mining(work_manager, &rpc_config).map_err(|e| "mining error")?;
    }

    let handler = || {
        let author = Author::new(rpc_config.clone());
        let state = State::new(rpc_config.clone());
        let system = System::new(rpc_config.clone());
        let chain = Chain::new(rpc_config.clone());

        let pow =  work_manger.clone().map(Pow::new);
        yee_switch_rpc_servers::rpc_handler::<_, _, _, _, _, yee_runtime::Hash, yee_runtime::BlockNumber>(
            author,
            state,
            system,
            chain,
            pow,
        )
    };


    let _server = yee_switch_rpc_servers::start_http(&rpc_address_http, handler())?;

    info!(target: TARGET, "Switch rpc http listen on: {}", rpc_address_http);

    let _server = yee_switch_rpc_servers::start_ws(&rpc_address_ws, handler())?;

    info!(target: TARGET, "Switch rpc ws listen on: {}", rpc_address_ws);


    exit.wait().unwrap();

    signal.fire();

    Ok(())
}

fn parse_address(
    address: &str,
    port: Option<u16>,
) -> error::Result<SocketAddr> {
    let mut address: SocketAddr = address.parse().map_err(
        |_| format!("Invalid address: {}", address)
    )?;
    if let Some(port) = port {
        address.set_port(port);
    }

    Ok(address)
}
