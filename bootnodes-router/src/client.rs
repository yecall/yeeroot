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

use serde_derive::{Deserialize, Serialize};
use jsonrpc_client_http::{self, HttpTransport, HttpHandle};
use substrate_cli::error;
use crate::BootnodesRouterConf;
use serde::private::ser::constrain;
use futures::future::err;
use log::warn;

jsonrpc_client!(pub struct BootnodesRouterClient {
    pub fn bootnodes(&mut self) -> RpcRequest<BootnodesRouterConf>;
});

fn client(uri: &str) -> error::Result<BootnodesRouterClient<HttpHandle>> {
    let handle = HttpTransport::new().standalone().and_then(|x| x.handle(&uri)).map_err(|e| format!("{}", e))?;

    Ok(BootnodesRouterClient::new(handle))
}

pub fn call<F: FnOnce(BootnodesRouterClient<HttpHandle>) -> error::Result<R> + Copy, R>(f: F, bootnodes_routers: &Vec<String>) -> error::Result<R> {

    let mut error : error::Error = error::ErrorKind::Msg("Bootnodes router client call error".to_string()).into();
    for uri in bootnodes_routers {
        let mut client = client(uri.as_str());

        match client {
            Ok(client) => {
                let result = f(client);

                match result {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        warn!("Bootnodes router client call error: {}", e);
                        error = e;
                        continue;
                    }
                }
            }
            Err(e) => {
                warn!("Bootnodes router client call error: {}", e);
                error = e;
                continue;
            }
        }
    }
    Err(error)
}