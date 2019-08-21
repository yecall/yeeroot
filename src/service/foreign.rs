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

use substrate_service::{FactoryFullConfiguration, ServiceFactory};
use crate::service::NodeConfig;
use log::info;
use yee_foreign_network as network;
use yee_foreign_network::identity::Keypair;
use yee_foreign_network::identify_specialization::ForeignIdentifySpecialization;
use yee_foreign_network::config::{Params as NetworkParams, NetworkConfiguration};
use yee_foreign_network::multiaddr::Protocol;
use yee_foreign_network::Service;
use yee_foreign_network::ProtocolId;
use yee_bootnodes_router::BootnodesRouterConf;
use std::iter;
use std::net::Ipv4Addr;
use parity_codec::alloc::collections::HashMap;

const DEFAULT_FOREIGN_PORT : u16 = 30334;
const DEFAULT_PROTOCOL_ID: &str = "sup";

pub struct Params{
    pub client_version: String,
    pub protocol_version: String,
    pub node_key_pair: Keypair,
    pub shard_num: u16,
    pub foreign_port: Option<u16>,
    pub bootnodes_router_conf: Option<BootnodesRouterConf>,
}

pub fn start_foreign_network<F: ServiceFactory>(param: Params){

    let peer_id = get_peer_id(&param.node_key_pair);
    info!("Start foreign network: ");
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
        identify_specialization: ForeignIdentifySpecialization::new(param.protocol_version.to_string(), param.shard_num),
    };

    let protocol_id = network::ProtocolId::from(DEFAULT_PROTOCOL_ID.as_bytes());

    network::Service::<F::Block, _>::new(
        network_params,
        protocol_id
    );
}

fn get_foreign_boot_nodes(bootnodes_router_conf: &Option<BootnodesRouterConf>) -> HashMap<u16, Vec<String>>{

    match bootnodes_router_conf{
        Some(bootnodes_router_conf) => {
            bootnodes_router_conf.shards.iter().map(|(k, v)| (k.parse().unwrap(), v.foreign.clone())).collect()
        },
        None => HashMap::new(),
    }
}

fn get_peer_id(node_key_pair: &Keypair) -> String {
    let public = node_key_pair.public();
    let peer_id = public.clone().into_peer_id();
    peer_id.to_base58()
}