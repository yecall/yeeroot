use rstd::vec::Vec;
use primitives::{traits::Zero, generic::Era, };
use parity_codec::{Decode, Compact, Input};


pub struct OriginTransfer<Address, Balance>{
    sender: Address,
    signature: Vec<u8>,
    index: Compact<u64>,
    era: Era,
    dest: Address,
    amount: Balance,
}

pub struct RelayTransfer<Address, Balance, Hash> {
    transfer: OriginTransfer<Address, Balance>,
    height: Compact<u64>,
    hash: Hash,
    parent: Hash,
    proof: Vec<u8>,
}

impl<Address, Balance> OriginTransfer<Address, Balance>
    where
        Address: Decode + Default + Clone,
        Balance: Decode + Zero + Clone
{
    pub fn decode(data: Vec<u8>) -> Option<Self> {
        let mut input = data.as_slice();
        if let Some(len) = Decode::decode(&mut input) {
            let _len: Vec<()> = len;
        } else {
            return None;
        }

        let mut version = 0;
        if let Some(v) = input.read_byte() {
            version = v;
        } else {
            return None;
        }
        let is_signed = version & 0b1000_0000 != 0;
        let version = version & 0b0111_1111;
        if version != 1u8 {
            return None;
        }

        let mut sender = Address::default();
        let mut signature = Vec::new();
        let mut index = Compact(0u64);
        let mut era = Era::Immortal;
        if is_signed {
            // sender
            if let Some(s) = Decode::decode(&mut input) {
                sender = s
            } else {
                return None;
            }
            // signature
            signature = input[..64].to_vec();
            input = &input[64..];
            // index
            if let Some(i) = Decode::decode(&mut input) {
                index = i;
            } else {
                return None;
            }
            // era
            if let Some(e) = Decode::decode(&mut input) {
                era = e;
            } else {
                return None;
            }
        }
        // module
        if let Some(m) = Decode::decode(&mut input) {
            let _module: Compact<u64> = m;
        } else {
            return None;
        }
        // function
        if let Some(f) = Decode::decode(&mut input) {
            let _func: Compact<u64> = f;
        } else {
            return None;
        }
        // dest address
        let mut dest = Address::default();
        if let Some(d) = Decode::decode(&mut input) {
            dest = d;
        } else {
            return None;
        }
        // amount
        let mut amount = Zero::zero();
        if let Some(a) = Decode::decode(&mut input) {
            amount = a;
        } else {
            return None;
        }
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

impl<Address, Balance, Hash> RelayTransfer<Address, Balance, Hash>
    where
        Address: Decode + Default + Clone,
        Balance: Decode + Zero + Clone,
        Hash:  Decode + Default
{
    pub fn decode(data: Vec<u8>) -> Option<Self>{
        let mut input = data.as_slice();
        if let Some(len) = Decode::decode(&mut input) {
            let _len: Vec<()> = len;
        } else {
            return None;
        }

        let mut version = 0;
        if let Some(v) = input.read_byte() {
            version = v;
        } else {
            return None;
        }
        let is_signed = version & 0b1000_0000 != 0;
        let version = version & 0b0111_1111;
        // has signed or version not satisfy
        if is_signed || version != 1u8 {
            return None;
        }
        // module
        if let Some(m) = Decode::decode(&mut input) {
            let _module: Compact<u64> = m;
        } else {
            return None;
        }
        // function
        if let Some(f) = Decode::decode(&mut input) {
            let _func: Compact<u64> = f;
        } else {
            return None;
        }
        // origin transfer
        let mut origin_transfer = Vec::new();
        if let Some(ot) = Decode::decode(&mut input) {
            origin_transfer = ot;
        } else {
            return None;
        }
        // which block's height the origin transfer in
        let mut height = Compact(0u64);
        if let Some(h) = Decode::decode(&mut input){
            height = h;
        }else{
            return None;
        }
        // block hash
        let mut hash = Hash::default();
        if let Some(h) = Decode::decode(&mut input) {
            hash = h;
        } else {
            return None;
        }
        // which block's parent hash the origin transfer in
        let mut parent = Hash::default();
        if let Some(h) = Decode::decode(&mut input){
            parent = h;
        } else {
            return None;
        }
        // proof
        let mut proof = Vec::new();
        if let Some(p) = Decode::decode(&mut input) {
            proof = p;
        } else {
            return None;
        }
        // decode origin transfer and build relay transfer
        if let Some(ot) = OriginTransfer::decode(origin_transfer){
            return Some(RelayTransfer{
                transfer: ot,
                height,
                hash,
                parent,
                proof,
            });
        }

        None
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