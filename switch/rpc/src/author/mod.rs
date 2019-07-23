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
//pub mod fetch;

use log::warn;
use client::{self, Client};
use parity_codec::{Encode, Decode};
use transaction_pool::{
	txpool::{
		ChainApi as PoolChainApi,
		BlockHash,
		ExHash,
		IntoPoolError,
		Pool,
		watcher::Status,
	},
};
use jsonrpc_derive::rpc;
use jsonrpc_pubsub::{typed::Subscriber, SubscriptionId};
use primitives::{Bytes, Blake2Hasher, H256};
use crate::rpc::futures::{Sink, Stream, Future};
use runtime_primitives::{generic, traits};
use crate::subscriptions::Subscriptions;
use log::info;
pub mod error;

#[cfg(test)]
mod tests;

use self::error::Result;


use yee_runtime::opaque::Block;
use yee_runtime::{Hash};
use futures::future::ok;


/// Substrate authoring RPC API
#[rpc]
pub trait AuthorApi<Hash, BlockHash> {
	/// RPC metadata
	type Metadata;


//	/// Submit hex-encoded extrinsic for inclusion in block.
//	#[rpc(name = "author_submitExtrinsic")]
//	fn submit_extrinsic(&self, extrinsic: Bytes) -> Result<Hash>;

	/// Returns all pending extrinsics, potentially grouped by sender.
	#[rpc(name = "author_pendingExtrinsics")]
	fn pending_extrinsics(&self) -> Result<Vec<Bytes>>;

}

/// Authoring API
pub struct Author<P> {
	sync: Arc<network::SyncProvider<P>>,

	/// Subscriptions manager
	subscriptions: Subscriptions,
}

impl <P>Author<P>  {
	/// Create new instance of Authoring API.
	pub fn new(
		subscriptions: Subscriptions,
		sync: Arc<network::SyncProvider<P>>,

	) -> Self {
		Author {
			sync,
			subscriptions,
		}
	}

}

impl <P> AuthorApi<ExHash<P>, BlockHash<P>> for Author<P> where
	P: PoolChainApi + Sync + Send + 'static,
	P::Block: traits::Block<Hash=H256>,
	P::Error: 'static,


{
	type Metadata = crate::metadata::Metadata;

//	fn submit_extrinsic(&self, ext: Bytes) -> Result<ExHash<P>> {
//		let xt = Decode::decode(&mut &ext[..]).ok_or(error::Error::from(error::ErrorKind::BadFormat))?;
//		info!{"i am   xt ---{:?}",xt};
//		let v =  xt.hash();
//		let g =generic::BlockId::hash(best_block_hash);
//
//
//		ok(v)
//
//
//	}

	fn pending_extrinsics(&self) -> Result<Vec<Bytes>> {

		let a:Vec<u8> = vec![33,33];
		let  x = Bytes(a);
		let v = vec![x];
		Ok(v)
	}

}
