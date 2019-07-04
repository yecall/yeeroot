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


pub use structopt::clap::App;
pub use substrate_cli::{GetLogFilter, AugmentClap, CoreParams};
use structopt::{StructOpt, clap::{AppSettings, SubCommand}};
use std::num::ParseIntError;



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




