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

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

mod service;
#[macro_use]
mod protocol;
mod chain;
mod util;
pub mod config;
pub mod error;
pub mod message;
pub mod identify_specialization;
mod vprotocol;

pub use service::{Service, NetworkMsg, SyncProvider, ExHashT};
pub use network_libp2p::{
    identity, multiaddr,
    ProtocolId, Severity, Multiaddr,
    NetworkState, NetworkStatePeer, NetworkStateNotConnectedPeer, NetworkStatePeerEndpoint,
    build_multiaddr, PeerId, PublicKey, IdentifyInfo, IdentifySpecialization, DefaultIdentifySpecialization,
};
pub use message::{generic as generic_message};
pub use error::Error;
#[doc(hidden)]
pub use runtime_primitives::traits::Block as BlockT;

#[cfg(test)]
mod test;
