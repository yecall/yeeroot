use rstd::vec::Vec;
use primitives::{traits::Zero, generic::Era};
use parity_codec::{Encode, Decode, Compact, Input};


pub struct OriginTransfer<Address, Balance> {
    pub sender: Address,
    pub signature: Vec<u8>,
    pub index: Compact<u64>,
    pub era: Era,
    pub dest: Address,
    pub amount: Balance,
}

pub struct RelayTransfer<Address, Balance, Hash> {
    pub transfer: OriginTransfer<Address, Balance>,
    pub number: Compact<u64>,
    pub hash: Hash,
    pub parent: Hash,
    pub proof: Vec<u8>,
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
            Some(v) => v,
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
            let _type = match input.read_byte() {
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
        } else {
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
        let dest: Address = match Decode::decode(&mut input) {
            Some(addr) => addr,
            None => return None
        };
        // amount
        let amount: Balance = match Decode::decode(&mut input) {
            Some(a) => {
                let a_c: Compact<u128> = a;
                let buf = a_c.0.encode();
                match Decode::decode(&mut buf.as_slice()) {
                    Some(am) => am,
                    None => return None
                }
            }
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

    pub fn sender(&self) -> Address {
        self.sender.clone()
    }

    pub fn dest(&self) -> Address {
        self.dest.clone()
    }

    pub fn amount(&self) -> Balance {
        self.amount.clone()
    }
}

impl<Address, Balance, Hash> RelayTransfer<Address, Balance, Hash>
    where
        Address: Decode + Default + Clone,
        Balance: Decode + Zero + Clone,
        Hash: Decode + Clone + Default
{
    pub fn decode(data: Vec<u8>) -> Option<Self> {
        let mut input = data.as_slice();
        // length
        let _len: Vec<()> = match Decode::decode(&mut input) {
            Some(len) => len,
            None => return None
        };
        // version
        let version = match input.read_byte() {
            Some(v) => v,
            None => return None
        };
        // is signed
        let is_signed = version & 0b1000_0000 != 0;
        let version = version & 0b0111_1111;
        // has signed or version not satisfy
        if is_signed || version != 1u8 {
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
        if input.len() < 64 + 32 + 32 + 2 {   // origin transfer min length
            return None;
        }
        // origin transfer
        let origin_transfer: Vec<u8> = match Decode::decode(&mut input) {
            Some(ot) => ot,
            None => return None
        };
        // which block's number the origin transfer in
        let number: Compact<u64> = match Decode::decode(&mut input) {
            Some(h) => h,
            None => return None
        };
        // block hash
        let block_hash: Hash = match Decode::decode(&mut input) {
            Some(h) => h,
            None => return None
        };
        // which block's parent hash the origin transfer in
        let parent: Hash = match Decode::decode(&mut input) {
            Some(h) => h,
            None => return None
        };
        // proof
        let proof: Vec<u8> = match Decode::decode(&mut input) {
            Some(p) => p,
            None => return None
        };
        // decode origin transfer and build relay transfer
        if let Some(ot) = OriginTransfer::decode(origin_transfer) {
            return Some(RelayTransfer {
                transfer: ot,
                number,
                hash: block_hash,
                parent,
                proof,
            });
        }

        None
    }

    pub fn number(&self) -> u64 {
        self.number.into()
    }

    pub fn hash(&self) -> Hash {
        self.hash.clone()
    }

    pub fn parent(&self) -> Hash {
        self.parent.clone()
    }

    pub fn sender(&self) -> Address {
        self.transfer.sender()
    }
}

#[test]
fn test_decode() {
    let tx = "250281ff784cb29a605b557c11a3e22520387c4377ded1734f56900d7f04946a0b70f338bc9b0ff2ffa4b95d4479cbaccefc7bbe908430f5c5ec571a25c71ee005d5755b65b7768dff90479a09f0d545384e57f057707664e2fa250818877a1a5a971f0f30000300ff8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48a10f";
    let data = hex::decode(tx).unwrap();
    let ot = OriginTransfer::decode(data).unwrap();
    let mut ot_a = ot.amount;
    let amount: u128 = Decode::decode(&mut ot_a).unwrap().into();

    assert_eq!(amount, 1000u128);
    //assert_eq!(ot.index, 1);
}
