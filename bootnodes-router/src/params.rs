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

use std::path::PathBuf;
use structopt::StructOpt;

pub const DEFAULT_BOOTNODES_ROUTER_PORT : u16 = 50001;

#[derive(Debug, StructOpt, Clone)]
pub struct BootnodesRouterCommandCmd {

    /// Specify TCP port.
    #[structopt(long = "port", value_name = "PORT")]
    pub port: Option<u16>,

    /// Specify custom base path.
    #[structopt(long = "base-path", short = "d", value_name = "PATH", parse(from_os_str))]
    pub base_path: Option<PathBuf>,

    /// Sets a custom logging filter
    #[structopt(short = "l", long = "log", value_name = "LOG_PATTERN")]
    pub log: Option<String>,

    /// Specify the development chain
    #[structopt(long = "dev")]
    pub dev: bool,

}

impl substrate_cli::GetLogFilter for BootnodesRouterCommandCmd {

    fn get_log_filter(&self) -> Option<String> {
        self.log.clone()
    }
}
