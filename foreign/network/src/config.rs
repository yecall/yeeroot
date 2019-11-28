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

//! Configuration for the networking layer of Substrate.

pub use network_libp2p::NetworkConfiguration;
use crate::chain::Client;
use crate::{IdentifySpecialization, ExHashT};
use runtime_primitives::traits::{Block as BlockT};
use std::sync::Arc;
use serde::export::PhantomData;

/// Service initialization parameters.
pub struct Params<B: BlockT, I: IdentifySpecialization> {
	/// Configuration.
	pub config: ProtocolConfig,
	/// Network layer configuration.
	pub network_config: NetworkConfiguration,
	/// Substrate relay chain access point.
	pub chain: Arc<Client<B>>,
	/// Identify specialization.
	pub identify_specialization: I,
}

/// Configuration for the Substrate-specific part of the networking layer.
#[derive(Clone)]
pub struct ProtocolConfig {
	/// self full node sharding number.
	pub shard_num: u16,
}
