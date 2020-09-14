use serde::{Deserialize, Serialize};
use parity_codec::Decode;
use crate::serde::SerdeHex;
use yee_assets::Call as AssetsCall;
use yee_balances::Call as BalancesCall;
use yee_relay::Call as RelayCall;
use yee_runtime::{
	AccountId,
	Call,
	Hash as RuntimeHash,
	UncheckedExtrinsic,
};
use yee_sr_primitives::{OriginExtrinsic, RelayParams};
use runtime_primitives::{
	generic::BlockId,
	traits::{Block as BlockT, BlakeTwo256, Hash, Header as HeaderT},
};
use substrate_primitives::{H256, Hasher};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RpcBlockResponse {
	pub block: Block,
	pub justification: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Block {
	pub header: Header,

	pub extrinsics: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Header {
	pub digest: Digest,
	#[serde(with = "SerdeHex")]
	pub extrinsics_root: Vec<u8>,
	#[serde(with = "SerdeHex")]
	pub number: u64,
	#[serde(with = "SerdeHex")]
	pub parent_hash: Vec<u8>,
	#[serde(with = "SerdeHex")]
	pub state_root: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Digest {
	pub logs: Vec<String>,
}

// impl Block {
// 	pub fn parent(&self) -> H256 {
// 		let p = hex::decode(self.header.parent_hash.clone()).expect("qed");
// 		p.as_slice().into()
// 	}
// }

pub fn decode_extrinsic(ec: Vec<u8>, tc: u16, cs: u16) -> (bool, Option<H256>) {
	let ex: UncheckedExtrinsic = Decode::decode(&mut ec.as_slice()).expect("qed");
	if ex.signature.is_some() {
		let h = BlakeTwo256::hash(ec.as_slice());
		match ex.function {
			Call::Balances(BalancesCall::transfer(dest, value)) => {
				let ds = yee_sharding_primitives::utils::shard_num_for(&dest, tc).expect("qed");
				if cs != ds {
					return (true, Some(h))
				}
			}
			Call::Assets(AssetsCall::transfer(_shard_code, id, dest, value)) => {
				let ds = yee_sharding_primitives::utils::shard_num_for(&dest, tc).expect("qed");
				if cs != ds {
					return (true, Some(h))
				}
			}
			_ => {}
		}
		return (false, None)
	} else {
		match ex.function {
			Call::Relay(RelayCall::transfer(_, otx, _, _, _)) => {
				let h = BlakeTwo256::hash(otx.as_slice());
				return (false, Some(h))
			}
			_ => {}
		}
	}

	return (true, None)
}