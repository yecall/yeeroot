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

use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use jsonrpc_core_client::TypedClient;
use crate::rpc::futures::{Future};
use crate::Config;
use crate::errors;
use rand::Rng;
use jsonrpc_client_transports::RpcError;
use impl_serde::serialize;
use num_bigint::BigUint;
use jsonrpc_core::BoxFuture;
use yee_serde_hex::SerdeHex;

pub struct RpcClient{
    config: Config,
}

impl RpcClient{

    pub fn new(config: Config) -> Self{

        RpcClient{
            config,
        }
    }

    fn get_random_rpc_uri(&self, shard_num: u16) -> errors::Result<String>  {

        let shard = self.config.shards.get(&format!("{}", shard_num)).ok_or(errors::Error::from(errors::ErrorKind::ConfigError))?;

        let rpc = &shard.rpc;

        if rpc.len()==0{
            return Err(errors::Error::from(errors::ErrorKind::ConfigError));
        }

        let mut rng =rand::thread_rng();

        let i = rng.gen_range(0, rpc.len());

        Ok(rpc[i].clone())

    }

    pub fn call_method<T: Serialize, R: DeserializeOwned + 'static>(
        &self, method: &str,
        returns: &'static str,
        args: T,
        shard_num: u16,
    ) -> errors::Result<R> {

        let uri = self.get_random_rpc_uri(shard_num)?;

        let result = jsonrpc_core_client::transports::http::connect(&uri)
            .and_then(|client: TypedClient| {
                client.call_method(method, returns, args).and_then(move |result| {
                    Ok(result)
                })
            }).wait().map_err(|e| {log::error!("RPC Client error: {:?}", e); e}).map_err(parse_error);

        result
    }

    pub fn call_method_async<T: Serialize + 'static + Send, R: DeserializeOwned + 'static + Send>(
        &self, method: &str,
        returns: &'static str,
        args: T,
        shard_num: u16,
    ) ->  errors::Result<BoxFuture<R>> {

        let uri = self.get_random_rpc_uri(shard_num)?;

        let method = method.to_owned();

        let run = jsonrpc_core_client::transports::http::connect(&uri)
            .and_then(move |client: TypedClient| {
                client.call_method(&method, "returns", args).and_then(move |result| {
                    Ok(result)
                })
            }).map_err(|e| {log::error!("RPC Client error: {:?}", e); e}).map_err(parse_error).map_err(|e|e.into());

        Ok(Box::new(run))
    }

}

fn parse_error(error: RpcError) -> errors::Error{

    errors::Error::from(errors::ErrorKind::RpcError(error))
}
