//! Service and ServiceFactory implementation. Specialized wrapper over Substrate service.

#![warn(unused_extern_crates)]

use std::sync::Arc;
use log::info;
use transaction_pool::{self, txpool::{Pool as TransactionPool}};
use substrate_service::{
	FactoryFullConfiguration, LightComponents, FullComponents, FullBackend,
	FullClient, LightClient, LightBackend, FullExecutor, LightExecutor,
	TaskExecutor,
};
use basic_authorship::ProposerFactory;
use substrate_client as client;
use primitives::{ed25519::Pair, Pair as PairT};
use inherents::InherentDataProviders;
use network::{construct_simple_protocol};
use substrate_executor::native_executor_instance;
use substrate_service::construct_service_factory;
use {
    parking_lot::RwLock,
    consensus::{import_queue, start_pow, PowImportQueue, JobManager, DefaultJob},
    foreign_chain::ForeignChain,
    yee_runtime::{
        self, GenesisConfig, opaque::Block, RuntimeApi,
        AccountId, AuthorityId, AuthoritySignature,
    },
    yee_rpc::{FullRpcHandlerConstructor, LightRpcHandlerConstructor},
    yee_sharding::identify_specialization::ShardingIdentifySpecialization,
};

mod sharding;
use sharding::prepare_sharding;

mod foreign;
use foreign::{Params, start_foreign_network};

pub use substrate_executor::NativeExecutor;
use yee_bootnodes_router::BootnodesRouterConf;
use yee_rpc::ProvideJobManager;

pub const IMPL_NAME : &str = "yee-node";
pub const NATIVE_PROTOCOL_VERSION : &str = "/yee/1.0.0";
pub const FOREIGN_PROTOCOL_VERSION : &str = "/yee-foreign/1.0.0";

// Our native executor instance.
native_executor_instance!(
	pub Executor,
	yee_runtime::api::dispatch,
	yee_runtime::native_version,
	include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/yee_runtime_wasm.compact.wasm")
);

#[derive(Clone)]
pub struct NodeConfig {
	inherent_data_providers: InherentDataProviders,
    pub coin_base: AccountId,
    pub shard_num: u16,
	pub foreign_port: Option<u16>,
	pub bootnodes_router_conf: Option<BootnodesRouterConf>,
    pub job_manager: Arc<RwLock<Option<Arc<JobManager<Job=DefaultJob<Block, <Pair as PairT>::Public>>>>>>,
}

impl Default for NodeConfig {
    fn default() -> Self{
        Self{
            inherent_data_providers: Default::default(),
            coin_base: Default::default(),
            shard_num: Default::default(),
            foreign_port: Default::default(),
            bootnodes_router_conf: Default::default(),
            job_manager: Arc::new(RwLock::new(None)),
        }
    }
}

impl ProvideJobManager<DefaultJob<Block, <Pair as PairT>::Public>> for NodeConfig{
    fn provide_job_manager(&self) -> Arc<RwLock<Option<Arc<JobManager<Job=DefaultJob<Block, <Pair as PairT>::Public>>>>>>{
        self.job_manager.clone()
    }
}

construct_simple_protocol! {
	/// Demo protocol attachment for substrate.
	pub struct NodeProtocol where Block = Block { }
}

construct_service_factory! {
	struct Factory {
		Block = Block,
		RuntimeApi = RuntimeApi,
		NetworkProtocol = NodeProtocol { |config| Ok(NodeProtocol::new()) },
		RuntimeDispatch = Executor,
		FullTransactionPoolApi = transaction_pool::ChainApi<client::Client<FullBackend<Self>, FullExecutor<Self>, Block, RuntimeApi>, Block>
			{ |config, client| Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client))) },
		LightTransactionPoolApi = transaction_pool::ChainApi<client::Client<LightBackend<Self>, LightExecutor<Self>, Block, RuntimeApi>, Block>
			{ |config, client| Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client))) },
		Genesis = GenesisConfig,
		Configuration = NodeConfig,
		FullService = FullComponents<Self>
			{ |config: FactoryFullConfiguration<Self>, executor: TaskExecutor|
				FullComponents::<Factory>::new(config, executor)
			},
		AuthoritySetup = {
			|service: Self::FullService, executor: TaskExecutor, key: Option<Arc<Pair>>| {
				if let Some(key) = key {
					info!("Using authority key {}", key.public());
					let proposer = Arc::new(ProposerFactory {
						client: service.client(),
						transaction_pool: service.transaction_pool(),
						inherents_pool: service.inherents_pool(),
					});
					let client = service.client();
                    executor.spawn(start_pow::<Self::Block, _, _, _, _, _, _, _>(
                        key.clone(),
						client.clone(),
						client,
						proposer,
						service.network(),
						service.on_exit(),
						service.config.custom.inherent_data_providers.clone(),
						service.config.custom.coin_base.clone(),
						service.config.custom.job_manager.clone(),
						service.config.force_authoring,
					)?);
				}

                //foreign network setup
                let config = &service.config;
				let foreign_network_param = Params{
                    client_version: config.network.client_version.clone(),
			        protocol_version : FOREIGN_PROTOCOL_VERSION.to_string(),
			        node_key_pair: config.network.node_key.clone().into_keypair().unwrap(),
			        shard_num: config.custom.shard_num,
			        foreign_port: config.custom.foreign_port,
			        bootnodes_router_conf: config.custom.bootnodes_router_conf.clone(),
			    };
                start_foreign_network::<Self, _>(foreign_network_param, service.client(), &executor).map_err(|e| format!("{:?}", e))?;

                // TODO: link with foreign_network
                let foreigh_chain = ForeignChain::<Self, FullClient<Self>>::new(
                    config,
                    config.custom.shard_num.into(),
                    service.client(),
                    executor,
                )?;
                // TODO: register to transaction pool

				Ok(service)
			}
		},
		LightService = LightComponents<Self>
			{ |config, executor| <LightComponents<Factory>>::new(config, executor) },
		FullImportQueue = PowImportQueue<Self::Block>
			{ |config: &mut FactoryFullConfiguration<Self> , client: Arc<FullClient<Self>>| {
			        prepare_sharding::<Self, _, _, AuthorityId, AuthoritySignature>(&config.custom, client.clone(), client.backend().to_owned())?;
					import_queue::<Self::Block, _, <Pair as PairT>::Public>(
						client.clone(),
						None,
						client,
						config.custom.inherent_data_providers.clone(),
					).map_err(Into::into)
				}
			},
		LightImportQueue = PowImportQueue<Self::Block>
			{ |config: &mut FactoryFullConfiguration<Self>, client: Arc<LightClient<Self>>| {
			        prepare_sharding::<Self, _, _, AuthorityId, AuthoritySignature>(&config.custom, client.clone(), client.backend().to_owned())?;
					import_queue::<Self::Block, _, <Pair as PairT>::Public>(
						client.clone(),
						None,
						client,
						config.custom.inherent_data_providers.clone(),
					).map_err(Into::into)
				}
			},
		FullRpcHandlerConstructor = FullRpcHandlerConstructor,
		LightRpcHandlerConstructor = LightRpcHandlerConstructor,
		IdentifySpecialization = ShardingIdentifySpecialization
		    { |config: &FactoryFullConfiguration<Self>| {
		        Ok(ShardingIdentifySpecialization::new(NATIVE_PROTOCOL_VERSION.to_string(), config.custom.shard_num))
		        }
		    },
	}
}
