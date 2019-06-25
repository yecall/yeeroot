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

//! Primitives for Yee POW

#![cfg_attr(not(feature = "std"), no_std)]

use runtime_primitives::ConsensusEngineId;

/// `ConsensusEngineId` of Yee POW consensus.
pub const YEE_POW_ENGINE_ID: ConsensusEngineId = [b'Y', b'e', b'e', b'!'];
