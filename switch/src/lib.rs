pub mod params;
use substrate_cli::{VersionInfo, error};

pub fn run_switch(cmd: params::SwitchCommandCmd) -> substrate_cli::error::Result<()> {
    println!("{}", cmd.switch_test.unwrap_or("".to_string()));

    Ok(())
}
