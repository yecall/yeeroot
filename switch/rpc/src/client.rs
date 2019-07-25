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

use substrate_cli;
use serde::Serialize;
use serde::de::DeserializeOwned;
use jsonrpc_core_client::TypedClient;
use std::time::Duration;
use crate::rpc::futures::{Future, Sink, Stream};
use crate::Config;
use crate::errors;
use crate::rpc;
use rand::Rng;
use jsonrpc_client_transports::RpcError;

pub struct RpcClient{
    config: Config,
}

impl RpcClient{

    pub fn new(config: Config) -> Self{

        RpcClient{
            config,
        }
    }

    fn get_random_rpc_uri(&self, shard_num: u16) -> rpc::Result<String>  {


        let rpc = match self.config.shards.get(&format!("{}", shard_num)) {
            Some(shard) => &shard.rpc,
            None => return Err(jsonrpc_core::Error::internal_error()),
        };

        if rpc.len()==0{
            return Err(jsonrpc_core::Error::internal_error());
        }

        let mut rng =rand::thread_rng();

        let i = rng.gen_range(0, rpc.len());

        Ok(rpc[i].clone())

    }

    pub fn call_method<T: Serialize, R: DeserializeOwned + 'static>(
        &self, method: &str,
        returns: &'static str,
        args: T,
    ) -> rpc::Result<R> {

        let uri = self.get_random_rpc_uri(0u16)?;

        let result = jsonrpc_core_client::transports::http::connect(&uri)
            .and_then(|client: TypedClient| {
                client.call_method(method, returns, args).and_then(move |result| {
                    Ok(result)
                })
            }).wait().map_err(|e| {log::error!("RPC Client error: {:?}", e); e}).map_err(parse_error);

        result
    }

}

fn parse_error(error: RpcError) -> jsonrpc_core::Error{

    match error{
        RpcError::JsonRpcError(e) => serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap(),
        other=> rpc::Error{
            code: rpc::ErrorCode::InternalError,
            message: rpc::ErrorCode::InternalError.description(),
            data: Some(format!("{:?}", other).into()),
        },
    }
}