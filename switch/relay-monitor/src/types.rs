use parity_codec::{Decode, Compact};
// use yee_sr_primitives::{OriginExtrinsic, RelayParams};
use runtime_primitives::{
    traits::{BlakeTwo256, Hash},
};
use serde::{Deserialize, Serialize};
use substrate_primitives::H256;

use yee_assets::Call as AssetsCall;
use yee_balances::Call as BalancesCall;
use yee_relay::Call as RelayCall;
use yee_runtime::{
    // AccountId,
    Call,
    // Hash as RuntimeHash,
    UncheckedExtrinsic,
};

use crate::serde::SerdeHex;

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

pub fn decode_extrinsic(ec: Vec<u8>, tc: u16, cs: u16) -> (bool, Option<H256>) {
    let ex: UncheckedExtrinsic = match Decode::decode(&mut ec.as_slice()){
        Some(v) => v,
        None => return (false, None)
    };
    if ex.signature.is_some() {
        let h = BlakeTwo256::hash(ec.as_slice());
        match ex.function {
            Call::Balances(BalancesCall::transfer(dest, _value)) => {
                let ds = yee_sharding_primitives::utils::shard_num_for(&dest, tc).expect("qed");
                if cs != ds {
                    return (true, Some(h));
                }
            }
            Call::Assets(AssetsCall::transfer(_shard_code, _id, dest, _value)) => {
                let ds = yee_sharding_primitives::utils::shard_num_for(&dest, tc).expect("qed");
                if cs != ds {
                    return (true, Some(h));
                }
            }
            _ => {}
        }
        return (false, None);
    } else {
        match ex.function {
            Call::Relay(RelayCall::transfer(_, otx, _, _, _)) => {
                let h = BlakeTwo256::hash(otx.as_slice());
                return (false, Some(h));
            }
            _ => {}
        }
    }

    return (true, None);
}