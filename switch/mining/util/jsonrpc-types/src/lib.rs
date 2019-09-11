mod bytes;
mod string;




#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct Timestamp(#[serde(with = "string")] pub u64);

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct Unsigned(#[serde(with = "string")] pub u64);




pub use self::bytes::JsonBytes;
pub use jsonrpc_core::types::{error, id, params, request, response, version};
pub use serde_derive::{Deserialize, Serialize};
