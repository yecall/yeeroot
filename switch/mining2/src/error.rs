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

use error_chain::*;

error_chain! {
	errors {
		/// Not implemented yet
		Unimplemented {
			description("not yet implemented"),
			display("Method Not Implemented"),
		}
		ShardsDown {
			description("all the shards are down"),
			display("All the shards are down"),
		}
		WorkExpired {
			description("work expired"),
			display("Work expired"),
		}
		ShardNotFound {
			description("shard not found"),
			display("Shard not found"),
		}
		TargetNotAccpect {
			description("target not accept"),
			display("Target not accept"),
		}
	}
}
