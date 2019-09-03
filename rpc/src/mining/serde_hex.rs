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

use std::{error, io};
use smallvec::SmallVec;
use serde::{Serializer, Deserializer};

pub trait SerdeHex : Sized{

    type Error : error::Error;

    fn uint_size() -> Option<usize>;

    fn into_bytes(&self) -> Result<Vec<u8>, Self::Error>;

    fn from_bytes(src: &[u8]) -> Result<Self, Self::Error>;

    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        use serde::ser::Error;
        let bytes = self.into_bytes().map_err(S::Error::custom)?;

        match Self::uint_size(){
            Some(size) => {
                impl_serde::serialize::serialize_uint(bytes.as_slice(), serializer)
            },
            _ => {
                impl_serde::serialize::serialize(bytes.as_slice(), serializer)
            }
        }
    }

    /// Attempt to deserialize a hexadecimal string into an instance of `Self`.
    fn deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        use serde::de::Error;

        let bytes = match Self::uint_size(){
            Some(size) => {
                impl_serde::serialize::deserialize_check_len(deserializer, impl_serde::serialize::ExpectedLen::Between(0, size))?
            },
            _ => {
                impl_serde::serialize::deserialize(deserializer)?
            }
        };
        Self::from_bytes(bytes.as_slice()).map_err(D::Error::custom)

    }
}

impl SerdeHex for u64{

    type Error = io::Error;

    fn uint_size() -> Option<usize>{
        Some(8)
    }

    fn into_bytes(&self) -> Result<Vec<u8>, Self::Error>{

        Ok(self.to_be_bytes().to_vec())
    }

    fn from_bytes(src: &[u8]) -> Result<Self, Self::Error>{

        Ok(0)
    }
}

impl SerdeHex for Vec<u8>{

    type Error = io::Error;

    fn uint_size() -> Option<usize>{
        None
    }

    fn into_bytes(&self) -> Result<Vec<u8>, Self::Error>{

        Ok(self.to_owned())
    }

    fn from_bytes(src: &[u8]) -> Result<Self, Self::Error>{

        Ok(src.to_vec())
    }
}