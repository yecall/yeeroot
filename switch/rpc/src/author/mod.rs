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

//! Substrate block-author/full-node API.

use std::sync::Arc;

use log::{warn, info};
use parity_codec::{Encode, Decode};
use jsonrpc_derive::rpc;
use jsonrpc_pubsub::{typed::Subscriber, SubscriptionId};
use primitives::{Bytes, Blake2Hasher, H256};
use runtime_primitives::{generic, traits};

pub mod error;

use self::error::Result;

/// Substrate authoring RPC API
#[rpc]
pub trait AuthorApi<Hash> {

	/// Submit hex-encoded extrinsic for inclusion in block.
	#[rpc(name = "author_submitExtrinsic")]
	fn submit_extrinsic(&self, extrinsic: Bytes) -> Result<Hash>;
}

/// Authoring API
pub struct Author {

}

impl Author{
	/// Create new instance of Authoring API.
	pub fn new() -> Self {
		Author {}
	}
}

impl<Hash> AuthorApi<Hash> for Author
{
	fn submit_extrinsic(&self, ext: Bytes) -> Result<Hash> {

		info!("submit_extrinsic");

		unimplemented!("");
	}

}
