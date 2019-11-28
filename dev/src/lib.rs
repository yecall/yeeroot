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
pub use network::NodeKeyConfig;
use primitives::H256;
use std::str::FromStr;

/// shard_num => (coinbase, rpc_port, ws_port, port, node_key, foreign_port)
/// pk:
/// tyee1jfakj2rvqym79lmxcmjkraep6tn296deyspd9mkh467u4xgqt3cqkv6lyl
///     a8666e483fd6c26dbb6deeec5afae765561ecc94df432f02920fc5d9cd4ae206ead577e5bc11215d4735cee89218e22f2d950a2a4667745ea1b5ea8b26bba5d6
///
/// tyee15zphhp8wmtupkf3j8uz5y6eeamkmknfgs6rj0hsyt6m8ntpvndvsmz3h3w
///     40e17c894e03256ea7cb671d79bcc88276c3fd6e6a05e9c0a9546c228d1f4955d8f18e85255020c97764251977b77f3b9e18f4d6de7b62522ab29a49cede669f
///
/// tyee14t6jxhs885azsd9v4t75cre9t4crv6a89q2vg8472u3tvwm3f94qgr9w77
///     708084bc9da56d9d1b201f50830269887ff2ef74e619c6af6ba7cf506068326f7cc9c4d646c531e83507928114ff9ef66350c62dfda3a7c5d2f0d9e0c37e7750
///
/// tyee12n2pjuwa5hukpnxjt49q5fal7m5h2ddtxxlju0yepzxty2e2fads5g57yd
///     0xa079ef650520662d08f270c4bc088f0c61abd0224f58243f6d1e6827c3ab234a7a1a0a3b89bbb02f2b10e357fd2a5ddb5050bc528c875a6990874f9dc6496772
///
/// tyee15c2cc2uj34w5jkfzxe4dndpnngprxe4nytaj9axmzf63ur4f8awq806lv6:
///     0xf8eb0d437140e458ec6103965a4442f6b00e37943142017e9856f3310023ab530a0cc96e386686f95d2da0c7fa423ab7b84d5076b3ba6e7756e21aaafe9d3696
///
/// tyee10n605lxn7k7rfm4t9nx3jd6lu790m30hs37j7dvm6jeun2kkfg7sf6fp9j
///     0xd0542cb78c304aa7ea075c93772d2a8283b75ea218eb9d6dd96ee181fc9da26caa746ccc1625cbd7451c25860c268792f57f108d536034173a42353ced9cf1e1
///
/// tyee16pa6aa7qnf6w5ztqdvla6kvmeg78pkmpd76d98evl88ppmarcctqdz5nu3:
///     0xa8f84e392246b1a4317b1deb904a8272c0428d3d324e1889be8f00b0500a1e63845dbc96f4726783d94d7edcdeb8878ce4dcac793c41e815942c664687599c19
///
/// tyee18z4vztn7d0t9290d6tmlucqcelj4d4luzshnfh274vsuf62gkdrsd7hqxh
///     0x2878ed7cdefe4cac9ae586531bce3dc74f9133793130efd8ada50a897654ef4c41ac8c027c3be243fdc77cbe3e9d3e8eea53f48b870936e13bda27842676d8de
pub const SHARD_CONF : [(u16, (&str, u16, u16, u16, &str, u16)); 8] = [
    (0, ("tyee1jfakj2rvqym79lmxcmjkraep6tn296deyspd9mkh467u4xgqt3cqkv6lyl", 9033, 9044, 30333, "0000000000000000000000000000000000000000000000000000000000000001", 30334)),
    (1, ("tyee15zphhp8wmtupkf3j8uz5y6eeamkmknfgs6rj0hsyt6m8ntpvndvsmz3h3w", 9133, 9144, 31333, "0000000000000000000000000000000000000000000000000000000000000002", 31334)),
    (2, ("tyee14t6jxhs885azsd9v4t75cre9t4crv6a89q2vg8472u3tvwm3f94qgr9w77", 9233, 9244, 32333, "0000000000000000000000000000000000000000000000000000000000000003", 32334)),
    (3, ("tyee12n2pjuwa5hukpnxjt49q5fal7m5h2ddtxxlju0yepzxty2e2fads5g57yd", 9333, 9344, 33333, "0000000000000000000000000000000000000000000000000000000000000004", 33334)),
    (4, ("tyee15c2cc2uj34w5jkfzxe4dndpnngprxe4nytaj9axmzf63ur4f8awq806lv6", 9433, 9444, 34333, "0000000000000000000000000000000000000000000000000000000000000005", 34334)),
    (5, ("tyee10n605lxn7k7rfm4t9nx3jd6lu790m30hs37j7dvm6jeun2kkfg7sf6fp9j", 9533, 9544, 35333, "0000000000000000000000000000000000000000000000000000000000000006", 35334)),
    (6, ("tyee16pa6aa7qnf6w5ztqdvla6kvmeg78pkmpd76d98evl88ppmarcctqdz5nu3", 9633, 9644, 36333, "0000000000000000000000000000000000000000000000000000000000000007", 36334)),
    (7, ("tyee18z4vztn7d0t9290d6tmlucqcelj4d4luzshnfh274vsuf62gkdrsd7hqxh", 9733, 9744, 37333, "0000000000000000000000000000000000000000000000000000000000000008", 37334)),
];

const BOOTNODES_ROUTER : &str = "http://127.0.0.1:50001";

pub struct RunParams{
    pub coinbase: String,
    pub rpc_port: u16,
    pub ws_port: u16,
    pub port: u16,
    pub node_key: String,
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

pub fn get_run_params(params_num: u16) -> error::Result<RunParams>{

    let shard_conf_map: HashMap<u16, (&str, u16, u16, u16, &str, u16)> = SHARD_CONF
        .iter().cloned().collect();

    let one = shard_conf_map.get(&params_num);
    let one = one.ok_or(error::ErrorKind::Msg("Invalid shard num".to_string().into()))?;

    let coinbase = one.0.to_string();
    let rpc_port = one.1;
    let ws_port = one.2;
    let port = one.3;
    let node_key = one.4.to_string();
    let foreign_port = one.5;

    let bootnodes_routers = vec![BOOTNODES_ROUTER.to_string()];

    Ok(RunParams{
        coinbase,
        rpc_port,
        ws_port,
        port,
        node_key,
        foreign_port,
        bootnodes_routers,
    })

}

pub fn get_switch_params(shard_count: Option<u16>) -> error::Result<Vec<SwitchParams>>{

    let shard_count = match shard_count{
        Some(8) => 8,
        None | Some(4) => 4,
        _ => return Err(error::ErrorKind::Msg("Invalid shard_count".into()).into())
    };

    let list = &SHARD_CONF[0..shard_count];

    let shard_conf_map: HashMap<u16, (&str, u16, u16, u16, &str, u16)> = list
        .iter().cloned().collect();

    Ok(shard_conf_map.iter().map(|(k, v)| SwitchParams{shard_num: *k, rpc_port: (*v).1} ).collect())
}

pub fn get_bootnodes_router_params(shard_count: Option<u16>)-> error::Result<Vec<BootnodesRouterParams>>{

    let shard_count = match shard_count{
        Some(8) => 8,
        None | Some(4) => 4,
        _ => return Err(error::ErrorKind::Msg("Invalid shard_count".into()).into())
    };

    let list = &SHARD_CONF[0..shard_count];

    let shard_conf_map: HashMap<u16, (&str, u16, u16, u16, &str, u16)> = list
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

pub fn parse_secp256k1_secret(hex: &String) -> error::Result<network::Secp256k1Secret> {
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
