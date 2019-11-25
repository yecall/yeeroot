use crate::service;
use futures::{future, Future, sync::oneshot};
use tokio::runtime::Runtime;
pub use substrate_cli::{VersionInfo, IntoExit, error};
use substrate_cli::{informant, parse_and_execute, TriggerExit};
use substrate_service::{ServiceFactory, Roles as ServiceRoles, Arc, FactoryFullConfiguration, FactoryBlock, FullClient};
use crate::chain_spec;
use std::ops::Deref;
use log::info;
use super::{
    custom_command::{run_custom_command, CustomCommand},
    custom_param::{YeeCliConfig, process_custom_args},
	dev_param::process_dev_param,
};
use parking_lot::{Mutex, RwLock};
use futures::sync::oneshot::Sender;
use signal_hook::{iterator::Signals, SIGUSR1, SIGINT, SIGTERM};
use std::thread;
use serde::export::fmt::Debug;
use crate::service::NodeConfig;
use runtime_primitives::{
	traits::{ProvideRuntimeApi, DigestItemFor, Block, Header},
};
use yee_sharding::{ShardingDigestItem, ScaleOutPhaseDigestItem};
use substrate_client::ChainHead;
use sharding_primitives::ShardingAPI;
use std::thread::sleep;
use std::time::Duration;

pub type FactoryBlockNumber<F> = <<FactoryBlock<F> as Block>::Header as Header>::Number;

/// Parse command line arguments into service configuration.
pub fn run<I, T, E>(args: I, exit: E, version: VersionInfo) -> error::Result<()> where
	I: IntoIterator<Item = T>,
	T: Into<std::ffi::OsString> + Clone,
	E: IntoExit<TriggerExit=CliTriggerExit<CliSignal>> + Clone,
	SignalOfExit<E>: Debug,
{
	parse_and_execute::<service::Factory, CustomCommand, YeeCliConfig, _, _, _, _, _>(
		load_spec, &version, service::IMPL_NAME, args, exit,
	 	|exit, custom_args, config| {

		    loop {
			    let signal = run_service::<_, service::Factory>(&version, exit.clone(), custom_args.clone(), config.clone());
			    match signal {
				    Ok(CliSignal::Restart) => {
					    sleep(Duration::from_secs(3));
					    info!("Restart service");
				    },
				    Ok(CliSignal::Stop) => return Ok(()),
				    Err(e) => return Err(e),
			    }
		    }
		}
	).and_then(run_custom_command::<service::Factory, _, _>).map_err(Into::into).map(|_| ())
}

fn run_service<E, F>(version: &VersionInfo, exit: E, mut custom_args: YeeCliConfig, mut config: FactoryFullConfiguration<F>) -> Result<SignalOfExit<E>, String>
where
	E: IntoExit<TriggerExit=CliTriggerExit<CliSignal>>,
	F: ServiceFactory<Configuration=NodeConfig<F>>,
	DigestItemFor<FactoryBlock<F>>: ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<FactoryBlockNumber<F>, u16>,
	FullClient<F>: ProvideRuntimeApi + ChainHead<FactoryBlock<F>>,
	<FullClient<F> as ProvideRuntimeApi>::Api: ShardingAPI<FactoryBlock<F>>,
{
	info!("{}", version.name);
	info!("  version {}", config.full_version());
	info!("  by {}, 2019", version.author);
	info!("Chain specification: {}", config.chain_spec.name());
	info!("Node name: {}", config.name);
	info!("Roles: {:?}", config.roles);

	process_dev_param::<F, FullClient<service::Factory>>(&mut config, &mut custom_args).map_err(|e| format!("{:?}", e))?;

	process_custom_args::<F, FullClient<service::Factory>>(&mut config, &custom_args).map_err(|e| format!("{:?}", e))?;

	let (exit, trigger_exit) = exit.into_exit();

	config.custom.trigger_exit = Some(Arc::new(trigger_exit));

	let runtime = Runtime::new().map_err(|e| format!("{:?}", e))?;
	let executor = runtime.executor();
	match config.roles {
		ServiceRoles::LIGHT => run_until_exit::<_, _, E>(
			runtime,
			F::new_light(config, executor).map_err(|e| format!("{:?}", e))?,
			exit
		),
		_ => run_until_exit::<_, _, E>(
			runtime,
			F::new_full(config, executor).map_err(|e| format!("{:?}", e))?,
			exit
		),
	}.map_err(|e| format!("{:?}", e))
}

fn load_spec(id: &str) -> Result<Option<chain_spec::ChainSpec>, String> {
	Ok(match chain_spec::Alternative::from(id) {
		Some(spec) => Some(spec.load()?),
		None => None,
	})
}

fn run_until_exit<T, C, E>(
	mut runtime: Runtime,
	service: T,
	e: E::Exit,
) -> error::Result<SignalOfExit<E>>
	where
		T: Deref<Target=substrate_service::Service<C>>,
		C: substrate_service::Components,
		E: IntoExit,
{
	let (exit_send, exit) = exit_future::signal();

	let executor = runtime.executor();
	informant::start(&service, exit.clone(), executor.clone());

	let signal = runtime.block_on(e);
	exit_send.fire();
	info!("Exit fired");

	// we eagerly drop the service so that the internal exit future is fired,
	// but we need to keep holding a reference to the global telemetry guard
	let _telemetry = service.telemetry();
	drop(service);
	signal.map_err(|_|"Signal error".into())
}

pub type SignalOfExit<E> = <<E as IntoExit>::TriggerExit as TriggerExit>::Signal;

// handles ctrl-c
#[derive(Clone)]
pub struct Exit;
impl IntoExit for Exit {
	type TriggerExit = CliTriggerExit<CliSignal>;
	type Exit = future::MapErr<oneshot::Receiver<<Self::TriggerExit as TriggerExit>::Signal>, fn(oneshot::Canceled) -> ()>;
	fn into_exit(self) -> (Self::Exit, Self::TriggerExit) {
		// can't use signal directly here because CtrlC takes only `Fn`.
		let (exit_send, exit) = oneshot::channel();

		let exit_send_cell = Arc::new(Mutex::new(Some(exit_send)));
		let exit_send_cell_clone = exit_send_cell.clone();

		let signals = Signals::new(&[SIGUSR1, SIGINT, SIGTERM]).unwrap();

		thread::spawn(move || {
			for sig in signals.forever() {
				info!("Received signal {:?}", sig);

				let signal = match sig{
					SIGUSR1 => CliSignal::Restart,
					_ => CliSignal::Stop,
				};

				if let Some(sender) = exit_send_cell.lock().take() {
					sender.send(signal).expect("Error sending exit signal");
				}
			}
		});

		(exit.map_err(drop), CliTriggerExit{sender: exit_send_cell_clone})
	}
}

#[derive(Debug)]
pub enum CliSignal{
	Stop,
	Restart,
}

pub struct CliTriggerExit<Signal>{
	sender: Arc<Mutex<Option<Sender<Signal>>>>,
}

impl<Signal> TriggerExit for CliTriggerExit<Signal>
where Signal: Send + Debug + 'static {
	type Signal = Signal;
	fn trigger_exit(&self, signal: Self::Signal){

		if let Some(sender) = self.sender.lock().take() {
			sender.send(signal).expect("Error sending exit signal");
		}
	}
}
