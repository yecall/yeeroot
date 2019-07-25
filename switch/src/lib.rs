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
use substrate_cli::{VersionInfo};
use crate::params::SwitchCommandCmd;
use log::{info, warn, debug, trace};
use std::thread;
use futures::future::Future;
use std::net::SocketAddr;
use yee_switch_rpc::author::Author;
use substrate_primitives::H256;

const TARGET : &str = "switch";

pub fn run(cmd: SwitchCommandCmd, version: VersionInfo) -> substrate_cli::error::Result<()> {

    let rpc_interface: &str = if cmd.rpc_external { "0.0.0.0" } else { "127.0.0.1" };

    let ws_interface: &str = if cmd.ws_external { "0.0.0.0" } else { "127.0.0.1" };

    let rpc_address_http = parse_address(&format!("{}:{}", rpc_interface, 9933), cmd.rpc_port)?;

    let rpc_address_ws = parse_address(&format!("{}:{}", ws_interface, 9944), cmd.ws_port)?;

    let handler = || {
        let author = Author::new();
        yee_switch_rpc_servers::rpc_handler::<_, H256>(
            author,
        )
    };

    let (signal, exit) = exit_future::signal();


    thread::Builder::new().name("switch_rpc_http".to_string()).spawn(move || {

        let server = yee_switch_rpc_servers::start_http(&rpc_address_http, handler()).unwrap();

        info!(target: TARGET, "Switch rpc http listen on: {}", rpc_address_http);

        server.wait();
    });


    thread::Builder::new().name("switch_rpc_ws".to_string()).spawn(move || {

        let server = yee_switch_rpc_servers::start_ws(&rpc_address_ws, handler()).unwrap();

        info!(target: TARGET, "Switch rpc ws listen on: {}", rpc_address_ws);

        server.wait();
    });

    exit.wait().unwrap();

    signal.fire();

    Ok(())
}

fn parse_address(
    address: &str,
    port: Option<u16>,
) -> substrate_cli::error::Result<SocketAddr> {
    let mut address: SocketAddr = address.parse().map_err(
        |_| format!("Invalid address: {}", address)
    )?;
    if let Some(port) = port {
        address.set_port(port);
    }

    Ok(address)
}




