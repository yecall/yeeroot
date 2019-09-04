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
use yee_foreign_network::config::{Params as NetworkParams, NetworkConfiguration};
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
use substrate_client::ChainHead;
use runtime_primitives::traits::{ProvideRuntimeApi, Header, Block};
use runtime_primitives::generic::BlockId;
use sharding_primitives::ShardingAPI;
use ansi_term::Colour;
use std::marker::PhantomData;
use primitives::H256;
use substrate_service::{Components, ComponentClient};

const DEFAULT_FOREIGN_PORT: u16 = 30334;
const DEFAULT_PROTOCOL_ID: &str = "sup";

pub struct Params {
    pub client_version: String,
    pub protocol_version: String,
    pub node_key_pair: Keypair,
    pub shard_num: u16,
    pub foreign_port: Option<u16>,
    pub bootnodes_router_conf: Option<BootnodesRouterConf>,
}

pub fn start_foreign_network<C>(param: Params, client: Arc<ComponentClient<C>>, executor: &TaskExecutor) -> error::Result<()> where
    <FactoryBlock<C::Factory> as Block>::Header: Header,
    C: Components,
    ComponentClient<C>: ProvideRuntimeApi + ChainHead<FactoryBlock<C::Factory>>,
    <ComponentClient<C> as ProvideRuntimeApi>::Api: ShardingAPI<FactoryBlock<C::Factory>>,
{
    let peer_id = get_peer_id(&param.node_key_pair);

    let shard_count = get_shard_count::<C::Factory, _>(&client)?;

    info!("Start foreign network: ");
    info!("  shard count: {}", shard_count);
    info!("  client version: {}", param.client_version);
    info!("  protocol version: {}", param.protocol_version);
    info!("  node key: {}", peer_id);
    info!("  shard num: {}", param.shard_num);
    info!("  foreign port: {:?}", param.foreign_port);
    info!("  bootnodes router conf: {:?}", param.bootnodes_router_conf);

    let port = match param.foreign_port {
        Some(port) => port,
        None => DEFAULT_FOREIGN_PORT,
    };

    let mut network_config = NetworkConfiguration::default();
    network_config.shard_num = param.shard_num;
    network_config.node_key_pair = param.node_key_pair;
    network_config.client_version = param.client_version;
    network_config.listen_addresses = vec![
        iter::once(Protocol::Ip4(Ipv4Addr::new(0, 0, 0, 0)))
            .chain(iter::once(Protocol::Tcp(port)))
            .collect()
    ];
    network_config.foreign_boot_nodes = get_foreign_boot_nodes(&param.bootnodes_router_conf);

    let network_params = NetworkParams {
        network_config,
        chain: client.clone(),
        identify_specialization: ForeignIdentifySpecialization::new(param.protocol_version.to_string(), param.shard_num),
    };

    let protocol_id = network::ProtocolId::from(DEFAULT_PROTOCOL_ID.as_bytes());

    let (service, _network_chan) = network::Service::<<C::Factory as ServiceFactory>::Block, _, H256>::new(
        network_params,
        protocol_id,
    ).map_err(|e| format!("{:?}", e))?;

    let task = Interval::new(Instant::now(), Duration::from_secs(3)).for_each(move |_instant| {
        info!(target: "foreign", "{}", get_status(&service.network_state(), shard_count));
        Ok(())
    }).map_err(|e| warn!("Foreign network error: {:?}", e));

    executor.spawn(task);

    Ok(())
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

fn get_shard_count<F, C>(client: &Arc<C>) -> error::Result<u16>
    where F: ServiceFactory,
          <FactoryBlock<F> as Block>::Header: Header,
          C: ProvideRuntimeApi + ChainHead<FactoryBlock<F>>,
          <C as ProvideRuntimeApi>::Api: ShardingAPI<FactoryBlock<F>>, {
    let api = client.runtime_api();
    let last_block_header = client.best_block_header()?;
    let last_block_id = BlockId::hash(last_block_header.hash());

    let shard_count = api.get_shard_count(&last_block_id).map(|x| x as u16).map_err(|e| format!("{:?}", e))?;

    Ok(shard_count)
}

fn get_status(network_state: &NetworkState, shard_count: u16) -> String {
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
        status.push_str(&format!("{} ({} peers) ", Colour::White.bold().paint(&format!("Shard#{}", i)), peer_count));
    }

    //remove last blank char
    if shard_count > 0 {
        status.remove(status.len() - 1);
    }
    status
}