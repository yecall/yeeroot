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

use crate as network;
use crate::service::Service;
use crate::ProtocolId;
use crate::config::NetworkConfiguration;
use crate::config::Params;
use crate::identify_specialization::ForeignIdentifySpecialization;
use crate::error::{self,Error};
use crate::IdentifySpecialization;
use crate::service::NetworkChan;
use crate::PeerId;
use crate::NodeKeyConfig;
use crate::Secp256k1Secret;
use crate::identity;
use crate::Secret;
use std::sync::Arc;
use runtime_primitives::{traits::{Block as BlockT, NumberFor}, ConsensusEngineId};
use yee_runtime::Block;
#[macro_use]
use primitives::H256;
use std::str::FromStr;

const DEFAULT_PROTOCOL_ID: &str = "sup";

#[test]
fn test() {

    let mut param = gen_param(0u16, "0000000000000000000000000000000000000000000000000000000000000001");

    let peer_id = get_peer_id(&param.network_config.node_key);

    let (network, _) = start_network::<Block, _>(param).unwrap();

    assert_eq!(peer_id, network.local_peer_id());
}

fn gen_param(shard_num: u16, node_key : &str) -> Params<ForeignIdentifySpecialization> {
    let mut network_config = NetworkConfiguration::new();
    network_config.node_key = NodeKeyConfig::Secp256k1(parse_secp256k1_secret(&node_key.to_string()).unwrap());

    let identify_specialization = ForeignIdentifySpecialization::new("yee-foreign/1.0.0".to_string(), shard_num);

    Params { network_config, identify_specialization }
}

fn start_network<B: BlockT + 'static, I: IdentifySpecialization>(params: Params<I>) -> Result<(Arc<Service<B, I>>, NetworkChan<B>), Error> {
    let protocol_id = ProtocolId::from(DEFAULT_PROTOCOL_ID.as_bytes());

    Service::new(
        params,
        protocol_id,
    )
}

fn get_peer_id(node_key_conf: &NodeKeyConfig) -> PeerId{
    let local_identity = node_key_conf.clone().into_keypair().unwrap();
    let local_public = local_identity.public();
    local_public.into_peer_id()
}

fn parse_secp256k1_secret(hex: &String) -> error::Result<network::Secp256k1Secret> {
    H256::from_str(hex).map_err(invalid_node_key).and_then(|bytes|
        network::identity::secp256k1::SecretKey::from_bytes(bytes)
            .map(network::Secret::Input)
            .map_err(invalid_node_key))
}

fn invalid_node_key(e: impl std::fmt::Display) -> error::Error {
    input_err(format!("Invalid node key: {}", e))
}

fn input_err<T: Into<String>>(msg: T) -> error::Error {
    error::ErrorKind::Msg(msg.into()).into()
}
