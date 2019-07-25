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

use crate::rpc;
use log::warn;

pub fn unimplemented() -> rpc::Error {
	rpc::Error {
		code: rpc::ErrorCode::ServerError(1),
		message: "Not implemented yet".into(),
		data: None,
	}
}

pub fn internal<E: ::std::fmt::Debug>(e: E) -> rpc::Error {
	warn!("Unknown error: {:?}", e);
	rpc::Error {
		code: rpc::ErrorCode::InternalError,
		message: "Unknown error occured".into(),
		data: Some(format!("{:?}", e).into()),
	}
}
