//! Service and ServiceFactory implementation. Specialized wrapper over Substrate service.

#![warn(unused_extern_crates)]

use std::time::Duration;
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
    consensus::{self, import_queue, start_pow, PowImportQueue, JobManager, DefaultJob},
    consensus_common::import_queue::ImportQueue,
    foreign_chain::{ForeignChain, ForeignChainConfig},
    substrate_service::{
        NetworkProviderParams, FactoryBlock, NetworkProvider, ServiceFactory,
    },
    yee_runtime::{
        self, GenesisConfig, opaque::Block, RuntimeApi,
        AccountId, AuthorityId, AuthoritySignature,
    },
    yee_rpc::{FullRpcHandlerConstructor, LightRpcHandlerConstructor},
    yee_sharding::identify_specialization::ShardingIdentifySpecialization,
};
use yee_primitives::{AddressCodec};
use substrate_cli::{TriggerExit};
use sharding_primitives::ScaleOut;

mod foreign;
use foreign::{start_foreign_network};

mod restarter;
use restarter::{start_restarter};

pub use substrate_executor::NativeExecutor;
use yee_bootnodes_router::BootnodesRouterConf;
use yee_rpc::ProvideJobManager;

use crfg;
use yee_primitives::Hrp;
use crate::cli::{CliTriggerExit, CliSignal};

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

/// Node specific configuration
pub struct NodeConfig<F: substrate_service::ServiceFactory> {
    /// crfg connection to import block
    // FIXME #1134 rather than putting this on the config, let's have an actual intermediate setup state
    pub crfg_import_setup: Option<(Arc<crfg::BlockImportForService<F>>, crfg::LinkHalfForService<F>)>,
    pub inherent_data_providers: InherentDataProviders,
    pub coinbase: AccountId,
    pub shard_num: u16,
    pub shard_count: u16,
    pub foreign_port: Option<u16>,
    pub bootnodes_router_conf: Option<BootnodesRouterConf>,
    pub job_manager: Arc<RwLock<Option<Arc<dyn JobManager<Job=DefaultJob<Block, <Pair as PairT>::Public>>>>>>,
    pub mine: bool,
    pub hrp: Hrp,
    pub scale_out: Option<ScaleOut>,
    pub trigger_exit: Option<Arc<CliTriggerExit<CliSignal>>>,
}

impl<F: substrate_service::ServiceFactory> Default for NodeConfig<F> {
    fn default() -> Self {
        Self {
            crfg_import_setup: None,
            inherent_data_providers: Default::default(),
            coinbase: Default::default(),
            shard_num: Default::default(),
            shard_count: Default::default(),
            foreign_port: Default::default(),
            bootnodes_router_conf: Default::default(),
            job_manager: Arc::new(RwLock::new(None)),
            mine: Default::default(),
            hrp: Default::default(),
            scale_out: Default::default(),
            trigger_exit: None,
        }
    }
}

impl<F: substrate_service::ServiceFactory> Clone for NodeConfig<F> {
    fn clone(&self) -> Self {
        Self {
            crfg_import_setup: None,
            coinbase: self.coinbase.clone(),
            shard_num: self.shard_num,
            shard_count: self.shard_count,
            foreign_port: self.foreign_port,
            mine: self.mine,
            hrp: self.hrp.clone(),
            scale_out: self.scale_out.clone(),
            trigger_exit: self.trigger_exit.clone(),

            // cloned config SHALL NOT SHARE some items with original config
            inherent_data_providers: Default::default(),
            bootnodes_router_conf: None,
            job_manager: Arc::new(RwLock::new(None)),
        }
    }
}

impl<F> ForeignChainConfig for NodeConfig<F> where F: substrate_service::ServiceFactory {
    fn get_shard_num(&self) -> u16 {
        self.shard_num
    }

    fn set_shard_num(&mut self, shard: u16) {
        self.shard_num = shard;
    }

    fn get_shard_count(&self) -> u16 {
        self.shard_count
    }
}

impl<F> ProvideJobManager<DefaultJob<Block, <Pair as PairT>::Public>> for NodeConfig<F> where F: substrate_service::ServiceFactory {
    fn provide_job_manager(&self) -> Arc<RwLock<Option<Arc<dyn JobManager<Job=DefaultJob<Block, <Pair as PairT>::Public>>>>>>{
        self.job_manager.clone()
    }
}

struct NetworkWrapper<F, EH> {
    inner: Arc<dyn NetworkProvider<F, EH>>,
}

impl<F, EH> Clone for NetworkWrapper<F, EH> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<F, EH> NetworkProvider<F, EH> for NetworkWrapper<F, EH> where
    F: ServiceFactory,
    EH: network::service::ExHashT,
{
    fn provide_network(
        &self,
        network_id: u32,
        params: NetworkProviderParams<F, EH>,
        protocol_id: network::ProtocolId,
        import_queue: Box<dyn ImportQueue<FactoryBlock<F>>>,
    ) -> Result<network::NetworkChan<FactoryBlock<F>>, network::Error> {
        self.inner.provide_network(network_id, params, protocol_id, import_queue)
    }
}

impl consensus::TriggerExit for CliTriggerExit<CliSignal>{
    fn trigger_restart(&self){
        self.trigger_exit(CliSignal::Restart);
    }

    fn trigger_stop(&self){
        self.trigger_exit(CliSignal::Stop);
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
        Configuration = NodeConfig<Self>,
        FullService = FullComponents<Self>
            { |config: FactoryFullConfiguration<Self>, executor: TaskExecutor|
                FullComponents::<Factory>::new(config, executor)
            },
        AuthoritySetup = {
            |mut service: Self::FullService, executor: TaskExecutor, key: Option<Arc<Pair>>| {
                let (block_import, link_half) = service.config.custom.crfg_import_setup.take()
                    .expect("Link Half and Block Import are present for Full Services or setup failed before. qed");

                // foreign network
                let config = &service.config;
                let foreign_network_param = foreign::Params{
                    client_version: config.network.client_version.clone(),
                    protocol_version : FOREIGN_PROTOCOL_VERSION.to_string(),
                    node_key_pair: config.network.node_key.clone().into_keypair().unwrap(),
                    shard_num: config.custom.shard_num,
                    shard_count: config.custom.shard_count,
                    foreign_port: config.custom.foreign_port,
                    bootnodes_router_conf: config.custom.bootnodes_router_conf.clone(),
                };
                let foreign_network = start_foreign_network::<FullComponents<Self>>(foreign_network_param, service.client(), &executor).map_err(|e| format!("{:?}", e))?;

                // foreign chain
                let foreign_network_wrapper = NetworkWrapper { inner: foreign_network.clone()};
                let foreigh_chain = ForeignChain::<Self>::new(
                    config,
                    foreign_network_wrapper,
                    executor.clone(),
                )?;

                // relay
                yee_relay::start_relay_transfer::<Self, _, _>(
                    service.client(),
                    &executor,
                    foreign_network.clone(),
                    Arc::new(foreigh_chain),
                    service.transaction_pool()
                ).map_err(|e| format!("{:?}", e))?;

                // restarter
                let restarter_param = restarter::Params{
                    authority_id: key.clone().map(|k|k.public()),
                    coinbase: config.custom.coinbase.clone(),
                    shard_num: config.custom.shard_num,
                    shard_count: config.custom.shard_count,
                    scale_out: config.custom.scale_out.clone(),
                    trigger_exit: config.custom.trigger_exit.clone(),
                };
                start_restarter::<FullComponents<Self>>(restarter_param, service.client(), &executor);

                // crfg
                let local_key = if service.config.disable_grandpa {
                    None
                } else {
                    key.clone()
                };

	            crfg::register_crfg_inherent_data_provider(
                    &service.config.custom.inherent_data_providers.clone(),
	                local_key.clone().unwrap().public()
	            )?;

                info!("Running crfg session as Authority {}", local_key.clone().unwrap().public().to_address(service.config.custom.hrp.clone()).expect("qed"));
                executor.spawn(crfg::run_crfg(
                    crfg::Config {
                        local_key,
                        // FIXME #1578 make this available through chainspec
                        gossip_duration: Duration::from_millis(333),
                        justification_period: 4096,
                        name: Some(service.config.name.clone())
                    },
                    link_half,
                    crfg::NetworkBridge::new(service.network()),
                    service.config.custom.inherent_data_providers.clone(),
                    service.on_exit(),
                )?);

                // pow
                if let Some(ref key) = key {
                    info!("Using authority key {}", key.public().to_address(service.config.custom.hrp.clone()).expect("qed"));
                    let proposer = Arc::new(ProposerFactory {
                        client: service.client(),
                        transaction_pool: service.transaction_pool(),
                        inherents_pool: service.inherents_pool(),
                    });
                    let client = service.client();

                    let params = consensus::Params{
                        force_authoring: service.config.force_authoring,
                        mine: service.config.custom.mine,
                        shard_extra: consensus::ShardExtra {
                            coinbase: service.config.custom.coinbase.clone(),
                            shard_num: service.config.custom.shard_num,
                            shard_count: service.config.custom.shard_count,
                            scale_out: service.config.custom.scale_out.clone(),
                            trigger_exit: service.config.custom.trigger_exit.clone().expect("qed"),
                        }
                    };

                    executor.spawn(start_pow::<Self::Block, _, _, _, _, _, _, _>(
                        key.clone(),
                        client.clone(),
                        block_import.clone(),
                        proposer,
                        service.network(),
                        service.on_exit(),
                        service.config.custom.inherent_data_providers.clone(),
                        service.config.custom.job_manager.clone(),
                        params,
                    )?);
                }

                Ok(service)
            }
        },
        LightService = LightComponents<Self>
            { |config, executor| <LightComponents<Factory>>::new(config, executor) },
        FullImportQueue = PowImportQueue<Self::Block>
            { |config: &mut FactoryFullConfiguration<Self> , client: Arc<FullClient<Self>>| {
                    let (block_import, link_half) = crfg::block_import::<_, _, _, RuntimeApi, FullClient<Self>>(
                        client.clone(), client.clone()
                    )?;

                    let block_import = Arc::new(block_import);
                    let justification_import = block_import.clone();
                    config.custom.crfg_import_setup = Some((block_import.clone(), link_half));

                    import_queue::<Self::Block, _, _, <Pair as PairT>::Public>(
                        block_import,
                        Some(justification_import),
                        client,
                        config.custom.inherent_data_providers.clone(),
                        consensus::ShardExtra {
                            coinbase: config.custom.coinbase.clone(),
                            shard_num: config.custom.shard_num,
                            shard_count: config.custom.shard_count,
                            scale_out: config.custom.scale_out.clone(),
                            trigger_exit: config.custom.trigger_exit.clone().expect("qed"),
                        }
                    ).map_err(Into::into)
                }
            },
        LightImportQueue = PowImportQueue<Self::Block>
            { |config: &mut FactoryFullConfiguration<Self>, client: Arc<LightClient<Self>>| {
                    import_queue::<Self::Block, _, _, <Pair as PairT>::Public>(
                        client.clone(),
                        None,
                        client,
                        config.custom.inherent_data_providers.clone(),
                        consensus::ShardExtra {
                            coinbase: config.custom.coinbase.clone(),
                            shard_num: config.custom.shard_num,
                            shard_count: config.custom.shard_count,
                            scale_out: config.custom.scale_out.clone(),
                            trigger_exit: config.custom.trigger_exit.clone().expect("qed"),
                        }
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
