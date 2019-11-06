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

//! setup default params for dev mode

mod error;
use std::collections::HashMap;
use std::iter;
use network::NodeKeyConfig;
use primitives::H256;
use std::str::FromStr;

/// shard_num => (coinbase, rpc_port, ws_port, port, node_key, foreign_port)
/// pk:
/// tyee15c2cc2uj34w5jkfzxe4dndpnngprxe4nytaj9axmzf63ur4f8awq806lv6:
///     0xf8eb0d437140e458ec6103965a4442f6b00e37943142017e9856f3310023ab530a0cc96e386686f95d2da0c7fa423ab7b84d5076b3ba6e7756e21aaafe9d3696
///
/// tyee10n605lxn7k7rfm4t9nx3jd6lu790m30hs37j7dvm6jeun2kkfg7sf6fp9j
///     0xd0542cb78c304aa7ea075c93772d2a8283b75ea218eb9d6dd96ee181fc9da26caa746ccc1625cbd7451c25860c268792f57f108d536034173a42353ced9cf1e1
///
/// tyee16pa6aa7qnf6w5ztqdvla6kvmeg78pkmpd76d98evl88ppmarcctqdz5nu3:
///     0xa8f84e392246b1a4317b1deb904a8272c0428d3d324e1889be8f00b0500a1e63845dbc96f4726783d94d7edcdeb8878ce4dcac793c41e815942c664687599c19
///
/// tyee12n2pjuwa5hukpnxjt49q5fal7m5h2ddtxxlju0yepzxty2e2fads5g57yd
///     0xa079ef650520662d08f270c4bc088f0c61abd0224f58243f6d1e6827c3ab234a7a1a0a3b89bbb02f2b10e357fd2a5ddb5050bc528c875a6990874f9dc6496772
pub const SHARD_CONF : [(u16, (&str, u16, u16, u16, &str, u16)); 4] = [
    (0, ("tyee15c2cc2uj34w5jkfzxe4dndpnngprxe4nytaj9axmzf63ur4f8awq806lv6", 9933, 9944, 30333, "0000000000000000000000000000000000000000000000000000000000000001", 30334)),
    (1, ("tyee10n605lxn7k7rfm4t9nx3jd6lu790m30hs37j7dvm6jeun2kkfg7sf6fp9j", 19933, 19944, 31333, "0000000000000000000000000000000000000000000000000000000000000002", 31334)),
    (2, ("tyee16pa6aa7qnf6w5ztqdvla6kvmeg78pkmpd76d98evl88ppmarcctqdz5nu3", 29933, 29944, 32333, "0000000000000000000000000000000000000000000000000000000000000003", 32334)),
    (3, ("tyee12n2pjuwa5hukpnxjt49q5fal7m5h2ddtxxlju0yepzxty2e2fads5g57yd", 39933, 39944, 33333, "0000000000000000000000000000000000000000000000000000000000000004", 33334)),
];

const BOOTNODES_ROUTER : &str = "http://127.0.0.1:50001";

pub struct RunParams{
    pub shard_num: u16,
    pub coinbase: String,
    pub rpc_port: u16,
    pub ws_port: u16,
    pub port: u16,
    pub node_key_config: NodeKeyConfig,
    pub foreign_port: u16,
    pub bootnodes_routers: Vec<String>,
}

pub struct SwitchParams{
    pub shard_num: u16,
    pub rpc_port: u16,
}

pub struct BootnodesRouterParams{
    pub shard_num: u16,
    pub port: u16,
    pub peer_id: String,
    pub foreign_port: u16,
}

pub fn get_run_params(shard_num: u16) -> error::Result<RunParams>{

    let shard_conf_map: HashMap<u16, (&str, u16, u16, u16, &str, u16)> = SHARD_CONF
        .iter().cloned().collect();

    let one = shard_conf_map.get(&shard_num);
    let one = one.ok_or(error::ErrorKind::Msg("Invalid shard num".to_string().into()))?;

    let coinbase = one.0.to_string();
    let rpc_port = one.1;
    let ws_port = one.2;
    let port = one.3;
    let node_key = one.4;
    let foreign_port = one.5;

    let node_key_config = NodeKeyConfig::Secp256k1(parse_secp256k1_secret(&node_key.to_string()).unwrap());

    let bootnodes_routers = vec![BOOTNODES_ROUTER.to_string()];

    Ok(RunParams{
        shard_num,
        coinbase,
        rpc_port,
        ws_port,
        port,
        node_key_config,
        foreign_port,
        bootnodes_routers,
    })

}

pub fn get_switch_params() -> error::Result<Vec<SwitchParams>>{

    let shard_conf_map: HashMap<u16, (&str, u16, u16, u16, &str, u16)> = SHARD_CONF
        .iter().cloned().collect();

    Ok(shard_conf_map.iter().map(|(k, v)| SwitchParams{shard_num: *k, rpc_port: (*v).1} ).collect())
}

pub fn get_bootnodes_router_params()-> error::Result<Vec<BootnodesRouterParams>>{

    let shard_conf_map: HashMap<u16, (&str, u16, u16, u16, &str, u16)> = SHARD_CONF
        .iter().cloned().collect();

    Ok(shard_conf_map.iter().map(|(k, v)| {
        let shard_num = *k;
        let port = (*v).3;
        let node_key = v.4;
        let node_key_config = NodeKeyConfig::Secp256k1(parse_secp256k1_secret(&node_key.to_string()).unwrap());
        let peer_id = get_peer_id(&node_key_config);
        let foreign_port = v.5;
        BootnodesRouterParams{
            shard_num, port, peer_id, foreign_port
        }
    } ).collect())
}

pub fn get_peer_id(node_key_config: &NodeKeyConfig) -> String {
    let public = node_key_config.clone().into_keypair().unwrap().public();
    let peer_id = public.clone().into_peer_id();
    peer_id.to_base58()
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
