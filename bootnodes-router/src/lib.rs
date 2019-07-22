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

use crate::params::BootnodesRouterCommandCmd;
use log::{info, trace};
use substrate_cli::{VersionInfo, error};
use std::path::{PathBuf, Path};
use app_dirs::{AppDataType, AppInfo};
use std::thread;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;
use jsonrpc_core::IoHandler;
use jsonrpc_derive::rpc;
use jsonrpc_http_server::ServerBuilder;
use jsonrpc_client_http::{self, HttpTransport, HttpHandle};
use serde_derive::{Deserialize, Serialize};
use futures::future::Future;
use error_chain::ChainedError;

pub mod params;
pub mod client;

#[macro_use]
extern crate jsonrpc_client_core;

const TARGET : &str = "bootnodes-router";

/// Run bootnodes router service
/// # Configure file description
/// ### Path
/// <base_path>/conf/bootnodes-router.toml
///
/// ### Content
/// ```
/// [shards]
/// [shards.0]
/// native = ["/ip4/127.0.0.1/tcp/60001/p2p/QmQZ8TjTqeDj3ciwr93EJ95hxfDsb9pEYDizUAbWpigtQN"]
/// foreign = ["/ip4/127.0.0.1/tcp/61001/p2p/QmQZ8TjTqeDj3ciwr93EJ95hxfDsb9pEYDizUAbWpigtQN"]
///
/// [shards.1]
/// native = ["/ip4/127.0.0.1/tcp/60011/p2p/QmXiB3jqqn2rpiKU7k1h7NJYeBg8WNSx9DiTRKz9ti2KSK"]
/// foreign = ["/ip4/127.0.0.1/tcp/61011/p2p/QmXiB3jqqn2rpiKU7k1h7NJYeBg8WNSx9DiTRKz9ti2KSK"]
/// ```
pub fn run(cmd: BootnodesRouterCommandCmd, version: VersionInfo) -> substrate_cli::error::Result<()> {

    let conf: BootnodesRouterConf = get_from_conf(&cmd, &version)?;

    info!(target: TARGET, "Bootnodes router_conf={:?}", conf);

    let port = cmd.port.unwrap_or(params::DEFAULT_BOOTNODES_ROUTER_PORT);

    let io = rpc_handler(conf);
    let addr = format!("0.0.0.0:{}", port);

    let (signal, exit) = exit_future::signal();

    let thread = thread::Builder::new().name("bootnodes_router".to_string()).spawn(move || {

        let server = ServerBuilder::new(io).
            threads(4).start_http(&addr.parse().unwrap()).unwrap();

        info!(target: TARGET, "Bootnodes router listen on: {}", addr);

        server.wait();

    });

    info!(target: TARGET, "Run bootnodes router successfully");

    exit.wait().unwrap();

    signal.fire();

    Ok(())
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct Shard {
    pub native: Vec<String>,
    pub foreign: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct BootnodesRouterConf {
    pub shards: HashMap<String, Shard>,
}

fn get_from_conf(cmd: &BootnodesRouterCommandCmd, version: &VersionInfo) -> substrate_cli::error::Result<BootnodesRouterConf> {
    let conf_path = conf_path(&base_path(cmd, version));

    let bootnodes_router_conf_path = conf_path.join("bootnodes-router.toml");

    trace!(target: TARGET, "conf_path:{}", bootnodes_router_conf_path.to_string_lossy());

    let mut file = File::open(&bootnodes_router_conf_path).map_err(|e|"Non-existed conf file")?;

    let mut str_val = String::new();
    file.read_to_string(&mut str_val)?;

    let conf: BootnodesRouterConf = toml::from_str(&str_val).map_err(|e| "Error reading conf file")?;

    Ok(conf)
}

pub fn conf_path(base_path: &Path) -> PathBuf {
    let mut path = base_path.to_owned();
    path.push("conf");
    path
}

fn base_path(cli: &BootnodesRouterCommandCmd, version: &VersionInfo) -> PathBuf {
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

fn rpc_handler(conf: BootnodesRouterConf) -> IoHandler<()> {
    let bootnodes_router_impl = BootnodesRouterImpl { conf };
    let mut io = jsonrpc_core::IoHandler::new();
    io.extend_with(bootnodes_router_impl.to_delegate());
    io
}

#[rpc]
pub trait BootnodesRouter {
    #[rpc(name = "bootnodes")]
    fn bootnodes(&self) -> jsonrpc_core::Result<BootnodesRouterConf>;
}

struct BootnodesRouterImpl {
    conf: BootnodesRouterConf,
}

impl BootnodesRouter for BootnodesRouterImpl {
    fn bootnodes(&self) -> jsonrpc_core::Result<BootnodesRouterConf> {
        Ok(self.conf.clone())
    }
}