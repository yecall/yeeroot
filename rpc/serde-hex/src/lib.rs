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
use serde::{Serializer, Deserializer, Serialize, Deserialize};
use num_bigint::BigUint;

pub trait SerdeHex: Sized {
    const DEFAULT_UINT_SIZE: usize = 0;

    type Error: error::Error;

    fn uint_size() -> Option<usize>;

    fn into_bytes(&self) -> Result<Vec<u8>, Self::Error>;

    fn from_bytes(src: &[u8]) -> Result<Self, Self::Error>;

    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        use serde::ser::Error;
        let bytes = self.into_bytes().map_err(S::Error::custom)?;

        match Self::uint_size() {
            Some(_size) => {
                impl_serde::serialize::serialize_uint(bytes.as_slice(), serializer)
            }
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

        let bytes = match Self::uint_size() {
            Some(size) => {
                impl_serde::serialize::deserialize_check_len(deserializer, impl_serde::serialize::ExpectedLen::Between(0, size))?
            }
            _ => {
                impl_serde::serialize::deserialize(deserializer)?
            }
        };
        Self::from_bytes(bytes.as_slice()).map_err(D::Error::custom)
    }
}

///Utils to wrap independent variable
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Hex<T : SerdeHex>(#[serde(with="SerdeHex")] pub T);

impl SerdeHex for u64 {
    const DEFAULT_UINT_SIZE: usize = 8;

    type Error = io::Error;

    fn uint_size() -> Option<usize> {
        Some(Self::DEFAULT_UINT_SIZE)
    }

    fn into_bytes(&self) -> Result<Vec<u8>, Self::Error> {
        Ok(self.to_be_bytes().to_vec())
    }

    fn from_bytes(src: &[u8]) -> Result<Self, Self::Error> {
        let mut bytes = [0u8; Self::DEFAULT_UINT_SIZE];

        let len = src.len();
        if len > Self::DEFAULT_UINT_SIZE {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        for i in 0..len {
            let j = Self::DEFAULT_UINT_SIZE - i - 1;
            bytes[j] = src[len - i - 1];
        }

        let u = u64::from_be_bytes(bytes);

        Ok(u)
    }
}

impl SerdeHex for u32 {
    const DEFAULT_UINT_SIZE: usize = 4;

    type Error = io::Error;

    fn uint_size() -> Option<usize> {
        Some(Self::DEFAULT_UINT_SIZE)
    }

    fn into_bytes(&self) -> Result<Vec<u8>, Self::Error> {
        Ok(self.to_be_bytes().to_vec())
    }

    fn from_bytes(src: &[u8]) -> Result<Self, Self::Error> {
        let mut bytes = [0u8; Self::DEFAULT_UINT_SIZE];

        let len = src.len();
        if len > Self::DEFAULT_UINT_SIZE {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        for i in 0..len {
            let j = Self::DEFAULT_UINT_SIZE - i - 1;
            bytes[j] = src[len - i - 1];
        }

        let u = u32::from_be_bytes(bytes);

        Ok(u)
    }
}

impl SerdeHex for u16 {
    const DEFAULT_UINT_SIZE: usize = 2;

    type Error = io::Error;

    fn uint_size() -> Option<usize> {
        Some(Self::DEFAULT_UINT_SIZE)
    }

    fn into_bytes(&self) -> Result<Vec<u8>, Self::Error> {
        Ok(self.to_be_bytes().to_vec())
    }

    fn from_bytes(src: &[u8]) -> Result<Self, Self::Error> {
        let mut bytes = [0u8; Self::DEFAULT_UINT_SIZE];

        let len = src.len();
        if len > Self::DEFAULT_UINT_SIZE {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        for i in 0..len {
            let j = Self::DEFAULT_UINT_SIZE - i - 1;
            bytes[j] = src[len - i - 1];
        }

        let u = u16::from_be_bytes(bytes);

        Ok(u)
    }
}

impl SerdeHex for Vec<u8> {
    type Error = io::Error;

    fn uint_size() -> Option<usize> {
        None
    }

    fn into_bytes(&self) -> Result<Vec<u8>, Self::Error> {
        Ok(self.to_owned())
    }

    fn from_bytes(src: &[u8]) -> Result<Self, Self::Error> {
        Ok(src.to_vec())
    }
}

impl SerdeHex for BigUint {
    type Error = io::Error;

    fn uint_size() -> Option<usize> {
        None
    }

    fn into_bytes(&self) -> Result<Vec<u8>, Self::Error> {
        Ok(self.to_bytes_be())
    }

    fn from_bytes(src: &[u8]) -> Result<Self, Self::Error> {
        Ok(BigUint::from_bytes_be(src))
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde::Deserialize;
    use crate::SerdeHex;
    use num_bigint::BigUint;

    #[test]
    fn test_de_u64() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct A {
            a: String,
            #[serde(with = "SerdeHex")]
            b: u64,
        }

        let s: A = serde_json::from_str(r#"{"a":"x", "b":"0xEF"}"#).unwrap();
        assert_eq!(s, A {
            a: "x".to_string(),
            b: 239,
        });

        let s: A = serde_json::from_str(r#"{"a":"x", "b":"0x0"}"#).unwrap();
        assert_eq!(s, A {
            a: "x".to_string(),
            b: 0,
        });

        let s: A = serde_json::from_str(r#"{"a":"x", "b":"0x01"}"#).unwrap();
        assert_eq!(s, A {
            a: "x".to_string(),
            b: 1,
        });

        let s: A = serde_json::from_str(r#"{"a":"x", "b":"0xFF1"}"#).unwrap();
        assert_eq!(s, A {
            a: "x".to_string(),
            b: 4081,
        });

        let s: A = serde_json::from_str(r#"{"a":"x", "b":"0xFFFFFFFFFFFFFFFF"}"#).unwrap();
        assert_eq!(s, A {
            a: "x".to_string(),
            b: 18446744073709551615,
        });
    }

    #[test]
    fn test_ser_u64() {
        #[derive(Debug, Serialize, PartialEq)]
        struct A {
            a: String,
            #[serde(with = "SerdeHex")]
            b: u64,
        }

        let s = serde_json::to_string(&A {
            a: "x".to_string(),
            b: 239,
        }).unwrap();
        assert_eq!(s, r#"{"a":"x","b":"0xef"}"#.to_string());

        let s = serde_json::to_string(&A {
            a: "x".to_string(),
            b: 239,
        }).unwrap();
        assert_eq!(s, r#"{"a":"x","b":"0xef"}"#.to_string());

        let s = serde_json::to_string(&A {
            a: "x".to_string(),
            b: 0,
        }).unwrap();
        assert_eq!(s, r#"{"a":"x","b":"0x0"}"#.to_string());

        let s = serde_json::to_string(&A {
            a: "x".to_string(),
            b: 1,
        }).unwrap();
        assert_eq!(s, r#"{"a":"x","b":"0x1"}"#.to_string());

        let s = serde_json::to_string(&A {
            a: "x".to_string(),
            b: 18446744073709551615,
        }).unwrap();
        assert_eq!(s, r#"{"a":"x","b":"0xffffffffffffffff"}"#.to_string());

    }

    #[test]
    fn test_de_biguint() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct A {
            a: String,
            #[serde(with = "SerdeHex")]
            b: BigUint,
        }

        let s: A = serde_json::from_str(r#"{"a":"x", "b":"0x64"}"#).unwrap();
        assert_eq!(s, A {
            a: "x".to_string(),
            b: BigUint::from(100u64),
        });
    }

    #[test]
    fn test_ser_biguint() {
        #[derive(Debug, Serialize, PartialEq)]
        struct A {
            a: String,
            #[serde(with = "SerdeHex")]
            b: BigUint,
        }

        let s = serde_json::to_string(&A {
            a: "x".to_string(),
            b: BigUint::from(100u64),
        }).unwrap();
        assert_eq!(s, r#"{"a":"x","b":"0x64"}"#.to_string());

    }
}