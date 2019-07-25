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

use substrate_cli::{VersionInfo, error};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{PathBuf, Path};
use app_dirs::{AppDataType, AppInfo};
use serde_derive::{Deserialize, Serialize};
use crate::params::SwitchCommandCmd;
use log::trace;

/// Switch config
/// # Configure file description
/// ### Path
/// <base_path>/conf/switch.toml
///
/// ### Content
/// ```
/// [shards]
/// [shards.0]
/// rpc = ["http://127.0.0.1:9933"]
///
/// [shards.1]
/// rpc = ["http://127.0.0.1:19933"]
/// ```
#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct Shard {
    pub rpc: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct SwitchConf {
    pub shards: HashMap<String, Shard>,
}

impl From<SwitchConf> for yee_switch_rpc::Config {
    fn from(conf: SwitchConf) -> Self {
        let mut shards: HashMap<String, yee_switch_rpc::Shard> = HashMap::new();

        for (k, v) in conf.shards {
            let shard = yee_switch_rpc::Shard{
                rpc: v.rpc,
            };
            shards.insert(k, shard);
        }

        yee_switch_rpc::Config{
            shards
        }
    }
}

pub fn get_config(cmd: &SwitchCommandCmd, version: &VersionInfo) -> substrate_cli::error::Result<SwitchConf> {
    let conf_path = conf_path(&base_path(cmd, version));

    let conf_path = conf_path.join("switch.toml");

    trace!(target: crate::TARGET, "conf_path:{}", conf_path.to_string_lossy());

    let mut file = File::open(&conf_path).map_err(|e| "Non-existed conf file")?;

    let mut str_val = String::new();
    file.read_to_string(&mut str_val)?;

    let conf: SwitchConf = toml::from_str(&str_val).map_err(|e| "Error reading conf file")?;

    Ok(conf)
}

fn conf_path(base_path: &Path) -> PathBuf {
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
