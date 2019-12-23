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

pub mod identify_specialization;
use parity_codec::{Encode, Decode};
use runtime_primitives::generic::DigestItem;

/// Generated module index in construct_runtime!
/// module specific log entries are prefixed by it and
///
/// MUST MATCH WITH construct_runtime MACRO ORDER
///
pub const GENERATED_MODULE_LOG_PREFIX: u8 = 2;
pub const GENERATED_SHARDING_PREFIX: u8 = 0;
pub const GENERATED_SCALE_OUT_PHASE_PREFIX: u8 = 1;

pub trait ShardingDigestItem<ShardNum>: Sized {
	fn sharding_info(num: ShardNum, cnt: ShardNum) -> Self;
	fn as_sharding_info(&self) -> Option<(ShardNum, ShardNum)>;
}

impl<ShardNum, Hash, AuthorityId, SealSignature> ShardingDigestItem<ShardNum> for DigestItem<Hash, AuthorityId, SealSignature> where
	ShardNum: Decode + Encode,
{
	fn sharding_info(num: ShardNum, cnt: ShardNum) -> Self {
		let prefix: [u8; 2] = [GENERATED_MODULE_LOG_PREFIX, GENERATED_SHARDING_PREFIX];
		let data = Encode::encode(&(prefix, num, cnt));
		DigestItem::Other(data)
	}

	fn as_sharding_info(&self) -> Option<(ShardNum, ShardNum)> {
		match self {
			DigestItem::Other(data) if data.len() >= 4
				&& data[0] == GENERATED_MODULE_LOG_PREFIX
				&& data[1] == GENERATED_SHARDING_PREFIX => {
				let input = &mut &data[2..];
				let num = Decode::decode(input)?;
				let cnt = Decode::decode(input)?;
				Some((num, cnt))
			}
			_ => None
		}
	}
}

pub trait ScaleOutPhaseDigestItem<BlockNumber, ShardNum>: Sized {
	fn scale_out_phase(scale_out_phase: ScaleOutPhase<BlockNumber, ShardNum>) -> Self;
	fn as_scale_out_phase(&self) -> Option<ScaleOutPhase<BlockNumber, ShardNum>>;
}

impl<BlockNumber, ShardNum, Hash, AuthorityId, SealSignature> ScaleOutPhaseDigestItem<BlockNumber, ShardNum> for DigestItem<Hash, AuthorityId, SealSignature> where
	ShardNum: Decode + Encode,
	BlockNumber: Decode + Encode,
{
	fn scale_out_phase(scale_out_phase: ScaleOutPhase<BlockNumber, ShardNum>) -> Self {
		let prefix: [u8; 2] = [GENERATED_MODULE_LOG_PREFIX, GENERATED_SCALE_OUT_PHASE_PREFIX];
		let data = Encode::encode(&(prefix, scale_out_phase));
		DigestItem::Other(data)
	}
	fn as_scale_out_phase(&self) -> Option<ScaleOutPhase<BlockNumber, ShardNum>>{
		match self {
			DigestItem::Other(data) if data.len() >= 4
				&& data[0] == GENERATED_MODULE_LOG_PREFIX
				&& data[1] == GENERATED_SCALE_OUT_PHASE_PREFIX => {
				let input = &mut &data[2..];
				let data = Decode::decode(input)?;
				Some(data)
			}
			_ => None
		}
	}
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum ScaleOutPhase<BlockNumber, ShardNum>{
	Started{
		observe_util: BlockNumber,
		shard_num: ShardNum,
	},
	NativeReady{
		observe_util: BlockNumber,
		shard_num: ShardNum,
	},
	Ready{
		observe_util: BlockNumber,
		shard_num: ShardNum,
	},
	Committing {
		shard_count: ShardNum,
	},
	Committed {
		shard_num: ShardNum,
		shard_count: ShardNum,
	},
}
