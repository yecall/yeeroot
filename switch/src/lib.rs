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

pub mod params;
use parity_codec::{Encode, Decode, Input};
use substrate_cli::{VersionInfo, error};
use crate::params::SwitchCommandCmd;
use log::{info, warn, debug, trace};
use std::io;
use std::thread;
use jsonrpc_derive::rpc;
use serde_derive::{Deserialize, Serialize};
use futures::future::Future;
use error_chain::ChainedError;
use jsonrpc_core::*;
use http::{hyper, Response, ServerBuilder};
use primitives::{Bytes, Blake2Hasher, H256, storage};
use yee_runtime::opaque::Block;
use route_rpc_servers::{self};
use jsonrpc_client_http::{self, HttpTransport};
//use test_client::runtime::Block;
use route_rpc::{system};

use yee_runtime::{Hash};
use tokio::runtime;
use std::sync::Arc;
use transaction_pool::{
    txpool::Pool,
    ChainApi,
};
/// Maximal payload accepted by RPC servers
const MAX_PAYLOAD: usize = 15 * 1024 * 1024;

pub fn route_run(cmd: SwitchCommandCmd, version: VersionInfo) -> substrate_cli::error::Result<()> {
    info!("switch cmd is :{:?}", cmd);

    //author_send();
    //get_extrinsic();
   // let subscriptions = rpc::apis::Subscriptions::new(task_executor.clone());

    let system = route_rpc::system::fetch::api(None);
    let handler = || {
        route_rpc_servers::rpc_handler::<Block,Hash,_>(
            system,

        )
    };

    let http_port = cmd.http_port.to_string();

    let ws_port = cmd.ws_port.to_string();


    let addr = format!("127.0.0.1:{}", ws_port);


        let server = route_rpc_servers::start_ws(&addr.parse().unwrap(), handler()).unwrap();

        info!("switch websocket  server is running   {}", "successfully!");

        server.wait();


//
//    let (signal, exit) = exit_future::signal();
//
//
//    thread::Builder::new().name("bootnodes_rter".to_string()).spawn(move || {
//        let addr = format!("127.0.0.1:{}", http_port);//let addrhehe = "127.0.0.1:8080".parse::<SocketAddr>()?;
//
//        let server = route_rpc_servers::start_http(&addr.parse().unwrap(), handler()).unwrap();
//
//        info!("switch http server is running   {}", "successfully!");
//
//        server.wait();
//    });
//
//
//    thread::Builder::new().name("bootnodes_router".to_string()).spawn(move || {
//        let addr = format!("127.0.0.1:{}", ws_port);
//
//        let server = route_rpc_servers::start_ws(&addr.parse().unwrap(), handler()).unwrap();
//
//        info!("switch websocket  server is running   {}", "successfully!");
//
//        server.wait();
//    });
//    exit.wait().unwrap();
//
//    signal.fire();

    Ok(())
}


#[macro_use]
extern crate jsonrpc_client_core;
jsonrpc_client!(pub struct SwitchClient {

   // pub fn author_submitExtrinsic(&mut self, param: String) -> RpcRequest<String>;
    pub fn author_submitExtrinsic(&mut self, extrinsic: Bytes) -> RpcRequest<String>;
});

fn author_send(ext: Bytes) {
    let transport = HttpTransport::new().standalone().unwrap();
    let transport_handle = transport
        .handle("http://127.0.0.1:9933")
        .unwrap();
    let mut client = SwitchClient::new(transport_handle);

    let result = client.author_submitExtrinsic(ext).call();
    println!("author_send--{:?}", &result);
    result.map_err(|e| format!("{:?}", e));
}




