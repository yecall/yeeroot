use rstd::vec::Vec;
use primitives::{traits::Zero, generic::Era};
use parity_codec::{Encode, Decode, Compact, Input};


pub struct OriginTransfer<Address, Balance> {
    sender: Address,
    signature: Vec<u8>,
    index: Compact<u64>,
    era: Era,
    dest: Address,
    amount: Balance,
}

impl<Address, Balance> OriginTransfer<Address, Balance>
    where
        Address: Decode + Default + Clone,
        Balance: Decode + Zero + Clone
{
    pub fn decode(data: Vec<u8>) -> Option<Self> {
        let mut input = data.as_slice();
        if input.len() < 64 + 1 + 1 {
            return None;
        }
        // length
        let _len: Vec<()> = match Decode::decode(&mut input) {
            Some(len) => len,
            None => return None
        };
        // version
        let version = match input.read_byte() {
            Some(v) =>v,
            None => return None
        };
        // is signed
        let is_signed = version & 0b1000_0000 != 0;
        let version = version & 0b0111_1111;
        if version != 1u8 {
            return None;
        }

        let (sender, signature, index, era) = if is_signed {
            // sender type
            let _type = match input.read_byte(){
                Some(a_t) => a_t,
                None => return None
            };
            // sender
            let sender = match Decode::decode(&mut input) {
                Some(s) => s,
                None => return None
            };
            if input.len() < 64 {
                return None;
            }
            // signature
            let signature = input[..64].to_vec();
            input = &input[64..];
            // index
            let index = match Decode::decode(&mut input) {
                Some(i) => i,
                None => return None
            };
            if input.len() < 1 {
                return None;
            }
            // era
            let era = if input[0] != 0u8 {
                match Decode::decode(&mut input) {
                    Some(e) => e,
                    None => return None
                }
            } else {
                input = &input[1..];
                Era::Immortal
            };
            (sender, signature, index, era)
        }else{
            (Address::default(), Vec::new(), Compact(0u64), Era::Immortal)
        };

        if input.len() < 2 + 32 + 1 {
            return None;
        }
        // module
        let _module: u8 = match input.read_byte() {
            Some(m) => m,
            None => return None
        };
        // function
        let _func: u8 = match input.read_byte() {
            Some(f) => f,
            None => return None
        };
        // dest address type
        let _type: u8 = match input.read_byte() {
            Some(t) => t,
            None => return None
        };
        // dest address
        let dest:Address  = match Decode::decode(&mut input) {
            Some(addr) => addr,
            None => return None
        };
        // amount
        let mut amount: Balance = match Decode::decode(&mut input) {
            Some(a) => {
                let a_c: Compact<u128> = a;
                let buf = a_c.0.encode();
                match Decode::decode(&mut buf.as_slice()) {
                    Some(am) => am,
                    None => return None
                }
            },
            None => return None

        };
        Some(OriginTransfer {
            sender,
            signature,
            index,
            era,
            dest,
            amount,
        })
    }

    pub fn dest(&self) -> Address {
        self.dest.clone()
    }

    pub fn amount(&self) -> Balance {
        self.amount.clone()
    }
}