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

use jsonrpc_client_http::{self, HttpTransport, HttpHandle};
use crate::BootnodesRouterConf;
use log::warn;
use crate::error;

jsonrpc_client!(pub struct BootnodesRouterClient {
    pub fn bootnodes(&mut self) -> RpcRequest<BootnodesRouterConf>;
});

fn client(uri: &str) -> error::Result<BootnodesRouterClient<HttpHandle>> {
    let handle = HttpTransport::new().standalone().and_then(|x| x.handle(&uri)).map_err(|e| format!("{}", e))?;

    Ok(BootnodesRouterClient::new(handle))
}

pub fn call<F: FnOnce(BootnodesRouterClient<HttpHandle>) -> error::Result<R> + Copy, R>(f: F, bootnodes_routers: &Vec<String>) -> error::Result<R> {

    let mut error : error::Error = "Bootnodes router client call error".into();
    for uri in bootnodes_routers {
        let client = client(uri.as_str());

        match client {
            Ok(client) => {
                let result = f(client);

                match result {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        warn!("Bootnodes router client call error");
                        error = e;
                        continue;
                    }
                }
            }
            Err(e) => {
                warn!("Bootnodes router client call error");
                error = e;
                continue;
            }
        }
    }
    Err(error)
}