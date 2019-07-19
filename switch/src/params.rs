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


use structopt::clap::App;
use substrate_cli::{GetLogFilter, AugmentClap, CoreParams};
use structopt::{StructOpt, clap::{AppSettings, SubCommand}};
use std::num::ParseIntError;
use std::path::PathBuf;



#[derive(Debug, StructOpt, Clone)]
pub struct SwitchCommandCmd {
    #[structopt(long = "switch-test")]
    pub switch_test: Option<String>,

   // #[structopt(long = "miner", parse(try_from_str = "parse_hex"))]
  //  pub miner: u32,

}

fn parse_hex(src: &str) -> Result<u32, ParseIntError> {
    u32::from_str_radix(src, 16)
}





#[derive(Debug, StructOpt, Clone)]

pub struct SwitchRouterCommandCmd {

    /// Specify http port.
    #[structopt(long = "http-port", short= "hp", value_name = "PORT",default_value = "9933")]
    pub http_port: u16,

    /// Specify ws port.
    #[structopt(long = "ws-port", short = "wp" ,value_name = "PORT",default_value = "9944")]
    pub ws_port: u16,

    /// Specify custom base path.
    #[structopt(long = "base-path", short = "b", value_name = "PATH", parse(from_os_str))]
    pub base_path: Option<PathBuf>,

    /// Sets a custom logging filter
    #[structopt(short = "l", long = "log", value_name = "LOG_PATTERN")]
    pub log: Option<String>,

    ///Specify max connections
    #[structopt(short = "m", long = "max connections", value_name = "MAX-CONNECTIONS" ,default_value = "100")]
    pub max_connections: u16,



}

impl substrate_cli::GetLogFilter for SwitchRouterCommandCmd {

    fn get_log_filter(&self) -> Option<String> {
        self.log.clone()
    }
}
