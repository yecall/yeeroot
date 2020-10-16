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
use structopt::{StructOpt, clap::{SubCommand}};
use yee_switch;
use yee_bootnodes_router;
use substrate_cli::VersionInfo;
use substrate_service::{ChainSpec, FactoryGenesis, ServiceFactory};
use crate::chain_revert::{RevertCmd, revert_chain};

//use yee_switch::SwitchCommandCmd;
#[derive(Clone, Debug)]
pub enum CustomCommand {
    SwitchCommandCmd(yee_switch::params::SwitchCommandCmd),
    BootnodesRouterCommandCmd(yee_bootnodes_router::params::BootnodesRouterCommandCmd),
    Revert(RevertCmd),
    None,
}

impl StructOpt for CustomCommand {
    fn clap<'a, 'b>() -> App<'a, 'b> {
        App::new("SwitchCommandCmd")
            .subcommand(
                yee_switch::params::SwitchCommandCmd::augment_clap(SubCommand::with_name("switch"))
                    .about("Yee switch model"))
            .subcommand(
                yee_bootnodes_router::params::BootnodesRouterCommandCmd::augment_clap(SubCommand::with_name("bootnodes-router"))
                    .about("Yee bootnodes router"))
            .subcommand(
                RevertCmd::augment_clap(SubCommand::with_name("revert"))
                    .about("Yee revert block")
            )
    }

    fn from_clap(matches: &::structopt::clap::ArgMatches) -> Self {
        match matches.subcommand() {
            ("switch", Some(matches)) =>
                CustomCommand::SwitchCommandCmd(yee_switch::params::SwitchCommandCmd::from_clap(matches)),
            ("bootnodes-router", Some(matches)) =>
                CustomCommand::BootnodesRouterCommandCmd(yee_bootnodes_router::params::BootnodesRouterCommandCmd::from_clap(matches)),
            ("revert", Some(matches)) =>
                CustomCommand::Revert(RevertCmd::from_clap(matches)),
            (_, Some(_)) => CustomCommand::None,
            (_, None) => CustomCommand::None,
        }
    }
}

impl GetLogFilter for CustomCommand {
    fn get_log_filter(&self) -> Option<String> {
        match self {
            CustomCommand::SwitchCommandCmd(cmd) => cmd.get_log_filter(),
            CustomCommand::BootnodesRouterCommandCmd(cmd) => cmd.get_log_filter(),
            CustomCommand::Revert(cmd) => cmd.get_log_filter(),
            CustomCommand::None => None
        }
    }
}

pub fn run_custom_command<F, E, S>(params : Option<(CustomCommand, S, E, VersionInfo)>) -> substrate_cli::error::Result<()> where
    F: ServiceFactory,
    S: FnOnce(&str) -> Result<Option<ChainSpec<FactoryGenesis<F>>>, String>,
{

    match params{
        Some((custom_command, spec_factory, _exit, version))=> match custom_command{
            CustomCommand::SwitchCommandCmd(cmd) => Ok(yee_switch::run(cmd, version).map_err(|e| format!("{:?}", e))?),
            CustomCommand::BootnodesRouterCommandCmd(cmd) => Ok(yee_bootnodes_router::run(cmd, version).map_err(|e| format!("{:?}", e))?),
            CustomCommand::Revert(cmd) => Ok(revert_chain::<F, S>(cmd, version, spec_factory).map_err(|e|format!("{:?}", e))?),
            CustomCommand::None => Ok(())
        },
        None => Ok(())
    }

}