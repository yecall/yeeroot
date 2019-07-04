pub mod params;


pub fn run_switch(cmd: params::SwitchCommandCmd) {
    println!("{}", cmd.switch_test.unwrap_or("".to_string()));

}
