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

use pow_primitives::PowTarget;
use {
	runtime_primitives::{
		traits::{NumberFor, Block as BlockT},
	},
};

#[derive(Clone, Debug)]
pub struct Context<Block> where
	Block: BlockT,
{
	pub genesis_pow_target: PowTarget,
	pub genesis_pow_target_adj : NumberFor<Block>,
	pub genesis_target_block_time: u64,
	pub genesis_shard_count: u16,
	pub genesis_scale_out_observe_blocks: NumberFor<Block>,
}
