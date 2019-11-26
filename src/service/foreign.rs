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

use substrate_service::{ServiceFactory, TaskExecutor, Arc, FactoryBlock};
use log::{info, warn};
use yee_foreign_network as network;
use yee_foreign_network::identity::Keypair;
use yee_foreign_network::identify_specialization::ForeignIdentifySpecialization;
use yee_foreign_network::config::{Params as NetworkParams, NetworkConfiguration, ProtocolConfig};
use yee_foreign_network::multiaddr::Protocol;
use yee_foreign_network::{SyncProvider, NetworkState};
use yee_bootnodes_router::BootnodesRouterConf;
use std::iter;
use std::net::Ipv4Addr;
use parity_codec::alloc::collections::HashMap;
use tokio::timer::Interval;
use std::time::{Instant, Duration};
use futures::stream::Stream;
use futures::future::Future;
use substrate_cli::error;
use substrate_client::{ClientInfo};
use ansi_term::Colour;
use substrate_service::{Components, ComponentClient, ComponentExHash};
use substrate_client::runtime_api::BlockT;

const DEFAULT_FOREIGN_PORT: u16 = 30334;
const DEFAULT_PROTOCOL_ID: &str = "sup";

pub struct Params {
    pub client_version: String,
    pub protocol_version: String,
    pub node_key_pair: Keypair,
    pub shard_num: u16,
    pub shard_count: u16,
    pub foreign_port: Option<u16>,
    pub bootnodes_router_conf: Option<BootnodesRouterConf>,
}

/// Start foreign network
///
/// Demo:
///
/// in service factory:
/// ```
/// let demo_param = foreign_demo::DemoParams{
///     shard_num: config.custom.shard_num,
/// };
/// foreign_demo::start_foreign_demo(demo_param, foreign_network, &executor).map_err(|e| format!("{:?}", e))?;
/// ```
///
/// in user mod:
/// ```
/// use yee_foreign_network as network;
/// use substrate_service::{TaskExecutor, Arc};
/// use substrate_cli::error;
/// use log::{info, warn};
/// use tokio::timer::Interval;
/// use std::time::{Instant, Duration, SystemTime, UNIX_EPOCH};
/// use futures::stream::Stream;
/// use futures::future::Future;
/// use yee_runtime::opaque::{Block, UncheckedExtrinsic};
/// use primitives::H256;
///
/// pub struct DemoParams {
///     pub shard_num: u16,
/// }
///
/// pub fn start_foreign_demo(
///     param: DemoParams,
///     foreign_network: Arc<network::SyncProvider<Block, H256>>,
///     executor: &TaskExecutor,
/// )
///     -> error::Result<()>
/// {
///     let status = foreign_network.network_state();
///
///     info!("foreign demo: status: {:?}", status);
///
///     let foreign_network_clone = foreign_network.clone();
///
///     let task = Interval::new(Instant::now(), Duration::from_secs(3)).for_each(move |_instant| {
///
///         let extrinsics = gen_extrinsics();
///
///         let target_shard_num = (param.shard_num + 1) % 4;
///
///         info!("foreign demo: sent relay extrinsics: shard_num: {} extrinsics: {:?}", target_shard_num, extrinsics);
///
///         foreign_network_clone.on_relay_extrinsics(target_shard_num, extrinsics);
///
///         Ok(())
///     }).map_err(|e| warn!("Foreign demo error: {:?}", e));
///
///     let message_task = foreign_network.out_messages().for_each(move |messages| {
///
///         info!("foreign demo: received messages: {:?}", messages);
///
///         Ok(())
///     });
///
///     executor.spawn(task);
///     executor.spawn(message_task);
///
///     Ok(())
/// }
///
/// fn gen_extrinsics() -> Vec<(H256, UncheckedExtrinsic)> {
///
///     let mut result = Vec::new();
///     for i in simple_rand_array(3) {
///
///         let hash = H256::from(i);
///         let extrinsic = UncheckedExtrinsic(i.to_vec());
///
///         result.push((hash, extrinsic))
///     }
///
///     result
/// }
///
/// fn simple_rand_array(count: usize) -> Vec<[u8; 32]>{
///     let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("qed").as_millis();
///
///     let mut result  = Vec::new();
///     for i in 0..count{
///         let tmp = now + i as u128;
///         let mut array = [0u8; 32];
///         array[0] = (tmp%256) as u8;
///         array[1] = (tmp/256%256) as u8;
///         array[3] = (tmp/256/256%256) as u8;
///
///         result.push(array);
///     }
///     result
/// }
/// ```
pub fn start_foreign_network<C>(param: Params, client: Arc<ComponentClient<C>>, executor: &TaskExecutor)
    -> error::Result<Arc<network::Service<FactoryBlock<C::Factory>, ForeignIdentifySpecialization, ComponentExHash<C>>>> where
    C: Components,
{
    let peer_id = get_peer_id(&param.node_key_pair);

    info!("Start foreign network: ");
    info!("  client version: {}", param.client_version);
    info!("  protocol version: {}", param.protocol_version);
    info!("  node key: {}", peer_id);
    info!("  shard num: {}", param.shard_num);
    info!("  shard count: {}", param.shard_count);
    info!("  foreign port: {:?}", param.foreign_port);
    info!("  bootnodes router conf: {:?}", param.bootnodes_router_conf);

    let port = match param.foreign_port {
        Some(port) => port,
        None => DEFAULT_FOREIGN_PORT,
    };

    let mut network_config = NetworkConfiguration::default();
    network_config.shard_num = param.shard_num;
    network_config.shard_count = param.shard_count;
    network_config.node_key_pair = param.node_key_pair;
    network_config.client_version = param.client_version;
    network_config.listen_addresses = vec![
        iter::once(Protocol::Ip4(Ipv4Addr::new(0, 0, 0, 0)))
            .chain(iter::once(Protocol::Tcp(port)))
            .collect()
    ];
    network_config.foreign_boot_nodes = get_foreign_boot_nodes(&param.bootnodes_router_conf);

    let config = ProtocolConfig{
        shard_num: param.shard_num,
    };

    let network_params = NetworkParams {
        config,
        network_config,
        chain: client.clone(),
        identify_specialization: ForeignIdentifySpecialization::new(param.protocol_version.to_string(), param.shard_num),
    };

    let protocol_id = network::ProtocolId::from(DEFAULT_PROTOCOL_ID.as_bytes());

    let (service, _network_chan) = network::Service::<<C::Factory as ServiceFactory>::Block, _, ComponentExHash<C>>::new(
        network_params,
        protocol_id,
    ).map_err(|e| format!("{:?}", e))?;

    let service_clone = service.clone();

    let shard_count = param.shard_count;

    let task = Interval::new(Instant::now(), Duration::from_secs(5)).for_each(move |_instant| {

        let network_state = service.network_state();
        let client_info = service.client_info();

        info!(target: "foreign", "{}", get_status(&network_state, &client_info, shard_count));
        Ok(())
    }).map_err(|e| warn!("Foreign network error: {:?}", e));

    executor.spawn(task);

    Ok(service_clone)
}

fn get_foreign_boot_nodes(bootnodes_router_conf: &Option<BootnodesRouterConf>) -> HashMap<u16, Vec<String>> {
    match bootnodes_router_conf {
        Some(bootnodes_router_conf) => {
            bootnodes_router_conf.shards.iter().map(|(k, v)| (k.parse().unwrap(), v.foreign.clone())).collect()
        }
        None => HashMap::new(),
    }
}

fn get_peer_id(node_key_pair: &Keypair) -> String {
    let public = node_key_pair.public();
    let peer_id = public.clone().into_peer_id();
    peer_id.to_base58()
}

fn get_status<B: BlockT>(network_state: &NetworkState, client_info: &HashMap<u16, Option<ClientInfo<B>>>, shard_count: u16) -> String {
    let mut result: HashMap<u16, u32> = HashMap::new();
    for (_peer_id, peer) in &network_state.connected_peers {
        match peer.shard_num {
            Some(shard_num) => {
                let count = result.entry(shard_num).or_insert(0);
                *count = *count + 1;
            }
            None => {}
        }
    }
    let mut status = String::new();
    for i in 0..shard_count {
        let peer_count = match result.get(&i) {
            Some(count) => *count,
            None => 0u32,
        };
        let best_number = match client_info.get(&i){
            Some(info) => match info{
                Some(info) => format!("{}", info.chain.best_number),
                None => "-".to_string(),
            },
            None => "-".to_string(),
        };
        status.push_str(&format!("{} (peers: {}, best: {}) ",
                                 Colour::White.bold().paint(&format!("Shard#{}", i)),
                                 peer_count,
                                 best_number,
        ));
    }

    //remove last blank char
    if shard_count > 0 {
        status.remove(status.len() - 1);
    }
    status
}