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
use crate::rpc;
use log::warn;
use jsonrpc_client_transports::RpcError;

error_chain! {

	links {
		Mining(mining::error::Error, mining::error::ErrorKind) #[doc="Mining error"];
	}

	errors {
		/// Not implemented yet
		Unimplemented {
			description("not yet implemented"),
			display("Method Not Implemented"),
		}
		ConfigError {
			description("config error"),
			display("Config error"),
		}
		InvalidShard {
			description("invalid shard"),
			display("Invalid shard"),
		}
		ParseError {
			description("parse error"),
			display("Parse error"),
		}
		InvalidAddress {
			description("invalid address"),
			display("Invalid address"),
		}
		InvalidShardCode {
			description("invalid shard code"),
			display("Invalid shard code"),
		}
		RpcError(e: jsonrpc_client_transports::RpcError) {
			description("rpc error"),
			display("Rpc error"),
		}
		GetWorkError {
			description("get work failed"),
			display("Get work failed"),
		}
		SumbitWorkError(reason: String) {
			description("submit work failed"),
			display("Submit work failed: {}", reason),
		}
	}
}


impl From<Error> for rpc::Error {
	fn from(e: Error) -> Self {
		match e {
			Error(ErrorKind::Unimplemented, _) => rpc::Error {
				code: rpc::ErrorCode::ServerError(1),
				message: "Not implemented yet".into(),
				data: None,
			},
			Error(ErrorKind::ConfigError, _) => rpc::Error {
				code: rpc::ErrorCode::ServerError(1),
				message: "Internal error".into(),
				data: None,
			},
			Error(ErrorKind::InvalidShard, _) => rpc::Error {
				code: rpc::ErrorCode::ServerError(1),
				message: "Invalid shard".into(),
				data: None,
			},
			Error(ErrorKind::ParseError, _) => rpc::Error {
				code: rpc::ErrorCode::ServerError(1),
				message: "Parse error".into(),
				data: None,
			},
			Error(ErrorKind::InvalidAddress, _) => rpc::Error {
				code: rpc::ErrorCode::ServerError(1),
				message: "Invalid address".into(),
				data: None,
			},
			Error(ErrorKind::RpcError(e), _) => match e{
				RpcError::JsonRpcError(e) => serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap(),
				other=> rpc::Error{
					code: rpc::ErrorCode::ServerError(1),
					message: rpc::ErrorCode::ServerError(1).description(),
					data: Some(format!("{:?}", other).into()),
				},
			}
			e => internal(e),
		}
	}
}

pub fn internal<E: ::std::fmt::Debug>(e: E) -> rpc::Error {
	warn!("Unknown error: {:?}", e);
	rpc::Error {
		code: rpc::ErrorCode::InternalError,
		message: rpc::ErrorCode::InternalError.description(),
		data: Some(format!("{:?}", e).into()),
	}
}
