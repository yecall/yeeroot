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
use yee_switch;

//use yee_switch::SwitchCommandCmd;
#[derive(Clone, Debug)]
pub enum CustomCommand {
    SwitchCommandCmd(yee_switch::params::SwitchCommandCmd),
    None,
}

impl StructOpt for CustomCommand {
    fn clap<'a, 'b>() -> App<'a, 'b> {
        App::new("SwitchCommandCmd")
            .subcommand(
                yee_switch::params::SwitchCommandCmd::augment_clap(SubCommand::with_name("switch"))
                    .about("yee switch model"))
    }

    fn from_clap(matches: &::structopt::clap::ArgMatches) -> Self {
        match matches.subcommand() {
            ("switch", Some(matches)) =>
                CustomCommand::SwitchCommandCmd(yee_switch::params::SwitchCommandCmd::from_clap(matches)),
            (_, Some(_)) => CustomCommand::None,
            (_, None) => CustomCommand::None,
        }
    }
}

impl GetLogFilter for CustomCommand {
    fn get_log_filter(&self) -> Option<String> {
        None
    }
}


pub fn run_custom_command(custom_command: Option<CustomCommand>) {
    if let Some(custom_command) = custom_command {
        match custom_command {
            CustomCommand::SwitchCommandCmd(switch_command_cmd) => yee_switch::run_switch(switch_command_cmd),

            CustomCommand::None => {}
        }
    }
}