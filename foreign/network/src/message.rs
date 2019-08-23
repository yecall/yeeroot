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

//! Network packet message types. These get serialized and put into the lower level protocol payload.

use bitflags::bitflags;
use runtime_primitives::{traits::{Block as BlockT, Header as HeaderT}};
use parity_codec::{Encode, Decode, Input, Output};

/// Type alias for using the message type using block type parameters.
pub type Message<B> = generic::Message<
	<B as BlockT>::Extrinsic
>;

/// A set of transactions.
pub type Transactions<E> = Vec<E>;

/// Generic types.
pub mod generic {
	use parity_codec::{Encode, Decode};
	use network_libp2p::CustomMessage;
	use super::Transactions;

	/// A network message.
	#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
	pub enum Message<Extrinsic> {
		/// Transactions.
		Transactions(Transactions<Extrinsic>),
	}

	impl<Extrinsic> CustomMessage for Message<Extrinsic>
		where Self: Decode + Encode
	{
		fn into_bytes(self) -> Vec<u8> {
			self.encode()
		}

		fn from_bytes(bytes: &[u8]) -> Result<Self, ()> {
			Decode::decode(&mut &bytes[..]).ok_or(())
		}
	}
}
