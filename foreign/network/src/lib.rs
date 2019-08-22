
mod service;
#[macro_use]
mod protocol;
mod util;
pub mod config;
pub mod error;
pub mod message;
pub mod identify_specialization;

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
