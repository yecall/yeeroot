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
use primitives::{ed25519::Pair, Pair as PairT, H256};
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
        AccountId,
    },
    yee_rpc::{FullRpcHandlerConstructor, LightRpcHandlerConstructor},
    yee_sharding::identify_specialization::ShardingIdentifySpecialization,
};
use substrate_cli::{TriggerExit};
use sharding_primitives::ScaleOut;
use runtime_primitives::traits::{Header, Block as BlockT, NumberFor};
use futures::sync::mpsc;
use yee_primitives::RecommitRelay;

mod foreign;
use foreign::{start_foreign_network};

mod restarter;
use restarter::{start_restarter};

pub use substrate_executor::NativeExecutor;
use yee_bootnodes_router::BootnodesRouterConf;
use yee_rpc::ProvideRpcExtra;

use crfg;
use yee_primitives::Hrp;
use crate::{CliTriggerExit, CliSignal};
use yee_context::{Context};
use crfg::CrfgState;
use yee_runtime::BlockNumber;

pub const IMPL_NAME : &str = "yee-node";
pub const NATIVE_PROTOCOL_VERSION : &str = "/yee/1.0.0";
pub const FOREIGN_PROTOCOL_VERSION : &str = "/yee-foreign/1.0.0";

#[cfg(feature = "custom-wasm-code")]
pub const WASM_CODE: &'static [u8] = include_bytes!(env!("WASM_CODE_PATH"));

#[cfg(not(feature = "custom-wasm-code"))]
pub const WASM_CODE: &'static [u8] = include_bytes!("../../runtime/wasm/target/wasm32-unknown-unknown/release/yee_runtime_wasm.compact.wasm");

// Our native executor instance.
native_executor_instance!(
    pub Executor,
    yee_runtime::api::dispatch,
    yee_runtime::native_version,
    WASM_CODE
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
    pub recommit_relay_sender: Arc<Option<mpsc::UnboundedSender<RecommitRelay<<F::Block as BlockT>::Hash>>>>,
    pub crfg_state: Arc<RwLock<Option<CrfgState<<F::Block as BlockT>::Hash, NumberFor<F::Block>>>>>,
    pub mine: bool,
    pub foreign_chains: Arc<RwLock<Option<ForeignChain<F>>>>,
    pub hrp: Hrp,
    pub scale_out: Option<ScaleOut>,
    pub trigger_exit: Option<Arc<dyn consensus::TriggerExit>>,
    pub context: Option<Context<F::Block>>,
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
            recommit_relay_sender: Arc::new(None),
            crfg_state: Arc::new(RwLock::new(None)),
            mine: Default::default(),
            foreign_chains: Arc::new(RwLock::new(None)),
            hrp: Default::default(),
            scale_out: Default::default(),
            trigger_exit: Default::default(),
            context: Default::default(),
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
            context: self.context.clone(),

            // cloned config SHALL NOT SHARE some items with original config
            inherent_data_providers: Default::default(),
            bootnodes_router_conf: None,
            job_manager: Arc::new(RwLock::new(None)),
            recommit_relay_sender: Arc::new(None),
            crfg_state: Arc::new(RwLock::new(None)),
            foreign_chains: Arc::new(RwLock::new(None)),
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

impl<F> ProvideRpcExtra<<F::Block as BlockT>::Hash, NumberFor<F::Block>, DefaultJob<Block, <Pair as PairT>::Public>> for NodeConfig<F> where
    F: substrate_service::ServiceFactory,
{
    fn provide_job_manager(&self) -> Arc<RwLock<Option<Arc<dyn JobManager<Job=DefaultJob<Block, <Pair as PairT>::Public>>>>>>{
        self.job_manager.clone()
    }

    fn provide_recommit_relay_sender(&self) -> Arc<Option<mpsc::UnboundedSender<RecommitRelay<<F::Block as BlockT>::Hash>>>> {
        self.recommit_relay_sender.clone()
    }

    fn provide_crfg_state(&self) -> Arc<RwLock<Option<CrfgState<<F::Block as BlockT>::Hash, NumberFor<F::Block>>>>> {
        self.crfg_state.clone()
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
            |mut service: Self::FullService, executor: TaskExecutor, key: Option<Arc<Pair>>, next_key: Option<Arc<Pair>>| {
                let (block_import, link_half) = service.config.custom.crfg_import_setup.take()
                    .expect("Link Half and Block Import are present for Full Services or setup failed before. qed");
                let (sender, receiver): (mpsc::UnboundedSender<RecommitRelay<H256>>, mpsc::UnboundedReceiver<RecommitRelay<H256>>) = mpsc::unbounded();

                // foreign network
                // let config = &service.config;
                let foreign_network_param = foreign::Params{
                    client_version: service.config.network.client_version.clone(),
                    protocol_version : FOREIGN_PROTOCOL_VERSION.to_string(),
                    node_key_pair: service.config.network.node_key.clone().into_keypair().unwrap(),
                    shard_num: service.config.custom.shard_num,
                    shard_count: service.config.custom.shard_count,
                    foreign_port: service.config.custom.foreign_port,
                    bootnodes_router_conf: service.config.custom.bootnodes_router_conf.clone(),
                };
                let foreign_network = start_foreign_network::<FullComponents<Self>>(foreign_network_param, service.client(), &executor).map_err(|e| format!("{:?}", e))?;

                // foreign chain
                let foreign_network_wrapper = NetworkWrapper { inner: foreign_network.clone()};
                let foreign_chain = ForeignChain::<Self>::new(
                    &service.config,
                    foreign_network_wrapper,
                    executor.clone(),
                )?;
                {
                    let mut config_foreign_chains = service.config.custom.foreign_chains.write();
                    *config_foreign_chains = Some(foreign_chain);
                }
                service.config.custom.recommit_relay_sender = Arc::new(Some(sender));

                // relay
                yee_relay::start_relay_transfer::<Self, _, _>(
                    service.client(),
                    &executor,
                    foreign_network.clone(),
                    service.config.custom.foreign_chains.clone(),
                    service.transaction_pool(),
                    receiver
                ).map_err(|e| format!("{:?}", e))?;

                // restarter
                let restarter_param = restarter::Params{
                    authority_id: next_key.clone().map(|k|k.public()),
                    coinbase: service.config.custom.coinbase.clone(),
                    shard_num: service.config.custom.shard_num,
                    shard_count: service.config.custom.shard_count,
                    scale_out: service.config.custom.scale_out.clone(),
                    trigger_exit: service.config.custom.trigger_exit.clone().expect("qed"),
                };
                start_restarter::<FullComponents<Self>>(restarter_param, service.client(), &executor);

                // crfg
                let (local_key, local_next_key) = if service.config.disable_grandpa {
                    (None, None)
                } else {
                    (key.clone(), next_key.clone())
                };

                let worker_key = if local_next_key.is_some() { local_next_key.clone() } else { local_key.clone() };

	            crfg::register_crfg_inherent_data_provider(
                    &service.config.custom.inherent_data_providers.clone(),
	                worker_key.clone().unwrap().public()
	            )?;

                info!("Running crfg session as Authority {:?}", local_key.clone().unwrap().public());
                executor.spawn(crfg::run_crfg(
                    crfg::Config {
                        local_key,
                        local_next_key,
                        // FIXME #1578 make this available through chainspec
                        gossip_duration: Duration::from_millis(333),
                        justification_period: 4096,
                        name: Some(service.config.name.clone())
                    },
                    link_half,
                    crfg::NetworkBridge::new(service.network()),
                    service.config.custom.inherent_data_providers.clone(),
                    service.on_exit(),
                    service.config.custom.crfg_state.clone(),
                )?);

                // pow
                if let Some(ref key) = worker_key {
                    info!("Using authority key {:?}", key.public());
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
                        },
                        context: service.config.custom.context.clone().expect("qed"),
                    };

                    executor.spawn(start_pow::<Self, Self::Block, _, _, _, _, _, _, _>(
                        key.clone(),
                        client.clone(),
                        block_import.clone(),
                        proposer,
                        service.network(),
                        service.on_exit(),
                        service.config.custom.inherent_data_providers.clone(),
                        service.config.custom.job_manager.clone(),
                        params,
                        service.config.custom.foreign_chains.clone(),
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

                    import_queue::<Self, _,  _, <Pair as PairT>::Public>(
                        block_import,
                        Some(justification_import),
                        client,
                        config.custom.inherent_data_providers.clone(),
                        config.custom.foreign_chains.clone(),
                        consensus::ShardExtra {
                            coinbase: config.custom.coinbase.clone(),
                            shard_num: config.custom.shard_num,
                            shard_count: config.custom.shard_count,
                            scale_out: config.custom.scale_out.clone(),
                            trigger_exit: config.custom.trigger_exit.clone().expect("qed"),
                        },
                        config.custom.context.clone().expect("qed"),
                    ).map_err(Into::into)
                }
            },
        LightImportQueue = PowImportQueue<Self::Block>
            { |config: &mut FactoryFullConfiguration<Self>, client: Arc<LightClient<Self>>| {
                    import_queue::<Self, _, _, <Pair as PairT>::Public>(
                        client.clone(),
                        None,
                        client,
                        config.custom.inherent_data_providers.clone(),
                        Arc::new(RwLock::new(None)),
                        consensus::ShardExtra {
                            coinbase: config.custom.coinbase.clone(),
                            shard_num: config.custom.shard_num,
                            shard_count: config.custom.shard_count,
                            scale_out: config.custom.scale_out.clone(),
                            trigger_exit: config.custom.trigger_exit.clone().expect("qed"),
                        },
                        config.custom.context.clone().expect("qed"),
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
