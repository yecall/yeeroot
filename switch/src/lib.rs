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
use std::path::{PathBuf, Path};
use app_dirs::{AppDataType, AppInfo};
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;
use serde_derive::{Deserialize, Serialize};

pub fn switch_run(cmd: SwitchCommandCmd, version: VersionInfo) -> substrate_cli::error::Result<()> {
    info!("switch cmd is :{:?}", cmd);


    let conf: SwitchRouterConf = get_shard_config(&cmd, &version)?;

    info!(target: TARGET, "switch router_conf={:?}", &conf);



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


const TARGET : &str = "switch";

/// Run switch  service
/// # Configure file description
/// ### Path
/// <base_path>/conf/switch-rpc-router.toml
///
/// ### Content
/// ```
/// [shards]
/// [shards.0]
/// rpc = ["192.168.1.0,192.168.1.1,192.168.1.2"]
///
/// [shards.1]
/// rpc = ["202.168.1.0,202.168.1.1,202.168.1.2"]
/// ```

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct Shard {
    pub rpc: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct SwitchRouterConf {
    pub shards: HashMap<String, Shard>,
}

fn get_shard_config(cmd: &SwitchCommandCmd, version: &VersionInfo) -> substrate_cli::error::Result<SwitchRouterConf> {
    let conf_path = conf_path(&base_path(cmd, version));

    let conf_path = conf_path.join("switch-rpc-router.toml");

    trace!(target: TARGET, "conf_path:{}", conf_path.to_string_lossy());

    let mut file = File::open(&conf_path).map_err(|e|"Non-existed conf file")?;

    let mut str_val = String::new();
    file.read_to_string(&mut str_val)?;

    let conf: SwitchRouterConf = toml::from_str(&str_val).map_err(|e| "Error reading conf file")?;

    Ok(conf)
}

pub fn conf_path(base_path: &Path) -> PathBuf {
    let mut path = base_path.to_owned();
    path.push("conf");
    path
}

fn base_path(cli: &SwitchCommandCmd, version: &VersionInfo) -> PathBuf {
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



