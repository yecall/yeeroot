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
use jsonrpc_core::*;
use yee_runtime::opaque::Block;
use route_rpc_servers::{self};
use yee_runtime::Hash;


pub fn route_run(cmd: SwitchCommandCmd, version: VersionInfo) -> substrate_cli::error::Result<()> {
    info!("switch cmd is :{:?}", cmd);
    let http_port = cmd.http_port.to_string();

    let ws_port = cmd.ws_port.to_string();

    let sysforhttp = route_rpc::system::fetch::api(None);
    let sysforws = route_rpc::system::fetch::api(None);

    let handlerhttp = || {
        route_rpc_servers::rpc_handler::<Block, Hash, _>(
            sysforhttp,
        )
    };

    let handlerws = || {
        route_rpc_servers::rpc_handler::<Block, Hash, _>(
            sysforws,
        )
    };

    let (signal, exit) = exit_future::signal();


    thread::Builder::new().name("switch-httpserver".to_string()).spawn(move || {
        let addr = format!("127.0.0.1:{}", http_port);//let addrhehe = "127.0.0.1:8080".parse::<SocketAddr>()?;

        let server = route_rpc_servers::start_http(&addr.parse().unwrap(), handlerhttp()).unwrap();

        info!("switch http server is running   {}", "successfully!");

        server.wait();
    });


    thread::Builder::new().name("switch-websocketserver".to_string()).spawn(move || {
        let addr = format!("127.0.0.1:{}", ws_port);

        let server = route_rpc_servers::start_ws(&addr.parse().unwrap(), handlerws()).unwrap();

        info!("switch websocket  server is running   {}", "successfully!");

        server.wait();
    });

    exit.wait().unwrap();

    signal.fire();

    Ok(())
}




