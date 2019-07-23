// Copyright 2017-2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Substrate RPC interfaces.

#![warn(missing_docs)]

mod errors;
mod helpers;
mod subscriptions;

pub use subscriptions::Subscriptions;

pub mod author;
pub mod metadata;
pub mod state;
pub mod system;

use jsonrpc_core as rpc;


use log::info;
use primitives::storage::{self, StorageKey, StorageData, StorageChangeSet};
use primitives::{Bytes, Blake2Hasher, H256};
use yee_runtime::Hash;
use jsonrpc_client_http::{self, HttpTransport, HttpHandle};
use yee_runtime::opaque::Block;

#[macro_use]
extern crate jsonrpc_client_core;

jsonrpc_client!(pub struct SwitchClient {
   // pub fn author_submitExtrinsic(&mut self, param: String) -> RpcRequest<String>;
    pub fn author_submitExtrinsic(&mut self, extrinsic: primitives::Bytes) -> RpcRequest<yee_runtime::Hash>;
    pub fn state_getStorage(&mut self,key: StorageKey, block:Hash) -> RpcRequest<Option<StorageData>>;
});

pub fn send_tx() {
    let transport = HttpTransport::new().standalone().unwrap();
    let transport_handle = transport
        .handle("http://127.0.0.1:9933")
        .unwrap();
    let mut client = SwitchClient::new(transport_handle);

//    let result = client.author_submitExtrinsic("0x310281ffd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d429bfe80a0b5d9b07f5c06b37d1a449328058a2b2c46c50fceb8997f2bb9f462a01dcb26b4e64bd12aa19dcb66281a57976722c06f397cfcafceda9a40fcc10b04f3000300ff1cbd2d43530a44705ad088af313e18f80b53ef16b36177cd4b77b846f2a5f07c7a110900"
//        .to_string()).call();
//    println!("author_send--{:?}", &result);
//    result.map_err(|e| format!("{:?}", e));
}

pub fn get_client() -> SwitchClient<HttpHandle> {
    let transport = HttpTransport::new().standalone().unwrap();
    let transport_handle = transport
        .handle("http://127.0.0.1:9933")
        .unwrap();
    SwitchClient::new(transport_handle)

}