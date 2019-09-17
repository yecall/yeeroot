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

pub trait ShardingDigestItem<N>: Sized {
	fn sharding_info(num: N, cnt: N) -> Self;
	fn as_sharding_info(&self) -> Option<(N, N)>;
}

impl<N, Hash, AuthorityId, SealSignature> ShardingDigestItem<N> for DigestItem<Hash, AuthorityId, SealSignature> where
	N: Decode + Encode,
{
	fn sharding_info(num: N, cnt: N) -> Self {
		let prefix: [u8; 2] = [GENERATED_MODULE_LOG_PREFIX, 0];
		let data = Encode::encode(&(prefix, num, cnt));
		DigestItem::Other(data)
	}

	fn as_sharding_info(&self) -> Option<(N, N)> {
		match self {
			DigestItem::Other(data) if data.len() >= 4
				&& data[0] == GENERATED_MODULE_LOG_PREFIX
				&& data[1] == 0 => {
				let input = &mut &data[2..];
				let num = Decode::decode(input)?;
				let cnt = Decode::decode(input)?;
				Some((num, cnt))
			}
			_ => None
		}
	}
}
