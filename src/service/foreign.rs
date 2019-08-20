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
use yee_foreign_network::identity::Keypair;

pub struct Params{
    pub node_key_pair: Keypair,
    pub shard_num: u16,
    pub bootnodes_routers: Vec<String>,
}

pub fn start_foreign_network(param: Params){

    let local_identity = param.node_key_pair;
    let local_public = local_identity.public();
    let local_peer_id = local_public.clone().into_peer_id();
    info!(target: "sub-libp2p", "Local node identity is: {}", local_peer_id.to_base58());

}