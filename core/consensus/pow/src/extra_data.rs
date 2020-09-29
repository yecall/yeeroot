// use std::fmt::Display;
use parity_codec::{Encode, Decode};
use serde::{Deserialize, Serialize};
use crate::big_array::BigArray;

#[derive(Clone, Encode, Decode, Serialize, Deserialize)]
pub struct ExtraData(#[serde(with = "BigArray")][u8; 40]);

impl ExtraData {
    pub fn from(input: [u8; 40]) -> Self{
        ExtraData(input)
    }
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl Default for ExtraData {
    fn default() -> Self {
        ExtraData([0u8;40])
    }
}

impl std::fmt::Debug for ExtraData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for ExtraData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let v = self.0.to_vec();
        let s = format!("{:?}", v);
        f.write_str(s.as_str())
    }
}

impl PartialEq for ExtraData {
    fn eq(&self, other: &ExtraData) -> bool {
        if self.0.to_vec().as_slice().cmp(other.0.to_vec().as_slice()) == std::cmp::Ordering::Equal {
            return true
        }
        return false
    }
}