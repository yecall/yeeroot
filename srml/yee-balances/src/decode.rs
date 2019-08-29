use rstd::vec::Vec;
use parity_codec::{Decode, Compact,Input};

struct OriginTransfer<Address, Signature> {
    sender: Address,
    signature: Signature,
    index: Compact<u64>,
    era: Vec<u8>,
    dest: Address,
    amount: Compact<u128>,
}

impl<Address, Signature>  OriginTransfer<Address, Signature>
    where
        Address: Decode + Default,
        Signature:Decode + Default
{
    fn decode(data: Vec<u8>) -> Option<Self> {
        if data.len() < 1 + 64 + 1 + 1 + 1 + 1 {
            return None;
        }
        let mut input = data.as_slice();
        let _len: Vec<()> = Decode::decode(&mut input).unwrap();
        let version = input.read_byte().unwrap();
        let is_signed = version & 0b1000_0000 != 0;
        let version = version & 0b0111_1111;
        if version != 1u8 {
            return None;
        }

        let mut sender = Address::default();
        let mut signature=Signature::default();
        let mut index = Compact(0u64);
        let mut era = Vec::new();
        if is_signed {
            sender = Decode::decode(&mut input).unwrap();
            // signature
            signature = Decode::decode(&mut input).unwrap();
            // index
           index = Decode::decode(&mut input).unwrap();
            // era
            let e = input.read_byte().unwrap();
            if e == 0 {
                era.push(e);
            } else {
                era = input[..2].to_vec();
                input = &input[2..];
            }
        }
        let module:Compact<u64> = Decode::decode(&mut input).unwrap();
        let func:Compact<u64> = Decode::decode(&mut input).unwrap();

        let dest:Address = Decode::decode(&mut input).unwrap();
        let mut amount: Compact<u128> = Decode::decode(&mut input).unwrap();
        Some(OriginTransfer {
            sender,
            signature,
            index,
            era,
            dest,
            amount,
        })
    }
}

#[test]
fn test_decode() {
    let tx = "250281ff784cb29a605b557c11a3e22520387c4377ded1734f56900d7f04946a0b70f338bc9b0ff2ffa4b95d4479cbaccefc7bbe908430f5c5ec571a25c71ee005d5755b65b7768dff90479a09f0d545384e57f057707664e2fa250818877a1a5a971f0f30000300ff8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48a10f";
    let data = hex::decode(tx).unwrap();
    let ot = OriginTransfer::decode(data).unwrap();
    let mut ot_a = ot.amount;
    //let amount: u128 = Decode::decode(&mut ot_a).unwrap().into();

    //assert_eq!(amount, 123);
    //assert_eq!(ot.index, 1);
}