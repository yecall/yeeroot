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

//! Libp2p network configuration.

use libp2p::identity::{Keypair, secp256k1, ed25519};
use libp2p::{Multiaddr, multiaddr::Protocol};
use std::error::Error;
use std::{io::{self, Write}, iter, fs, net::Ipv4Addr, path::{Path, PathBuf}};
use std::collections::HashMap;

/// Network service configuration.
#[derive(Clone)]
pub struct NetworkConfiguration {
	/// self full node Shard num
	pub shard_num: u16,
	/// Shard count
	pub shard_count: u16,
	/// Multiaddresses to listen for incoming connections.
	pub listen_addresses: Vec<Multiaddr>,
	/// Multiaddresses to advertise. Detected automatically if empty.
	pub public_addresses: Vec<Multiaddr>,
	/// List of initial node addresses
	pub foreign_boot_nodes: HashMap<u16, Vec<String>>,
	/// The node key configuration, which determines the node's network identity keypair.
	pub node_key_pair: Keypair,
	/// Maximum allowed number of incoming connections.
	pub in_peers: u32,
	/// Number of outgoing connections we're trying to maintain.
	pub out_peers: u32,
	/// Client identifier. Sent over the wire for debugging purposes.
	pub client_version: String,
	/// Name of the node. Sent over the wire for debugging purposes.
	pub node_name: String,
	/// If true, the network will use mDNS to discover other libp2p nodes on the local network
	/// and connect to them if they support the same chain.
	pub enable_mdns: bool,
}

impl Default for NetworkConfiguration {
	fn default() -> Self {
		NetworkConfiguration {
			shard_num: 0u16,
			shard_count: 0u16,
			listen_addresses: Vec::new(),
			public_addresses: Vec::new(),
			foreign_boot_nodes: HashMap::new(),
			node_key_pair: Keypair::generate_secp256k1(),
			in_peers: 5,
			out_peers: 15,
			client_version: "foreign-unknown".into(),
			node_name: "foreign-unknown".into(),
			enable_mdns: false,
		}
	}
}

impl NetworkConfiguration {
	/// Create a new instance of default settings.
	pub fn new() -> Self {
		Self::default()
	}

	/// Create new default configuration for localhost-only connection with random port (useful for testing)
	pub fn new_local() -> NetworkConfiguration {
		let mut config = NetworkConfiguration::new();
		config.listen_addresses = vec![
			iter::once(Protocol::Ip4(Ipv4Addr::new(127, 0, 0, 1)))
				.chain(iter::once(Protocol::Tcp(0)))
				.collect()
		];
		config
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tempdir::TempDir;

	fn secret_bytes(kp: &Keypair) -> Vec<u8> {
		match kp {
			Keypair::Ed25519(p) => p.secret().as_ref().iter().cloned().collect(),
			Keypair::Secp256k1(p) => p.secret().as_ref().iter().cloned().collect(),
			_ => panic!("Unexpected keypair.")
		}
	}

	#[test]
	fn test_secret_file() {
		let tmp = TempDir::new("x").unwrap();
		std::fs::remove_dir(tmp.path()).unwrap(); // should be recreated
		let file = tmp.path().join("x").to_path_buf();
		let kp1 = NodeKeyConfig::Ed25519(Secret::File(file.clone())).into_keypair().unwrap();
		let kp2 = NodeKeyConfig::Ed25519(Secret::File(file.clone())).into_keypair().unwrap();
		assert!(file.is_file() && secret_bytes(&kp1) == secret_bytes(&kp2))
	}

	#[test]
	fn test_secret_input() {
		let sk = secp256k1::SecretKey::generate();
		let kp1 = NodeKeyConfig::Secp256k1(Secret::Input(sk.clone())).into_keypair().unwrap();
		let kp2 = NodeKeyConfig::Secp256k1(Secret::Input(sk)).into_keypair().unwrap();
		assert!(secret_bytes(&kp1) == secret_bytes(&kp2));
	}

	#[test]
	fn test_secret_new() {
		let kp1 = NodeKeyConfig::Ed25519(Secret::New).into_keypair().unwrap();
		let kp2 = NodeKeyConfig::Ed25519(Secret::New).into_keypair().unwrap();
		assert!(secret_bytes(&kp1) != secret_bytes(&kp2));
	}
}

