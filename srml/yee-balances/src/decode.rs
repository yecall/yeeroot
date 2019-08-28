struct OriginTransfer {
    sender: Vec<u8>,
    signature: Vec<u8>,
    index: Vec<u8>,
    era: Vec<u8>,
    dest: Vec<u8>,
    amount: Vec<u8>,
}

impl OriginTransfer {
    fn decode(data: Vec<u8>) -> Option<Self> {
        if data.len() < 1 + 64 + 1 + 1 + 1 + 1 {
            return None;
        }
        let mut offset = 0;
        let version = data[offset];
        offset += 1;
        let is_signed = version & 0b1000_0000 != 0;
        let version = version & 0b0111_1111;
        if version != 1u8 {
            return None;
        }
        let mut signature = vec![];
        let mut sender = vec![];
        let mut index = vec![];
        let mut era = vec![];
        if is_signed {
            let b = data[offset];
            offset += 1;
            match b {
                x @ 0x00...0xef => {
                    sender = data[offset..offset + 1].to_vec();
                    offset += 1;
                }
                0xfc => {
                    sender = data[offset..offset + 2].to_vec();
                    offset += 2;
                }
                0xfd => {
                    sender = data[offset..offset + 4].to_vec();
                    offset += 4;
                }
                0xfe => { /* todo */ }
                0xff => {
                    sender = data[offset..offset + 32].to_vec();
                    offset += 32;
                }
                _ => { /* todo */ }
            };
            signature = data[offset..offset + 64].to_vec();
            offset += 64;
            index = data[offset..offset + 8].to_vec();
            offset += 8;
            let e = data[offset];
            if e == 0 {
                era.push(e);
                offset += 1;
            } else {
                era = data[offset..offset + 2].to_vec();
                offset += 2;
            }
        }
        let data = data[offset..].to_vec();
        offset = 0;
        let module = data[offset];
        offset += 1;
        let func = data[offset];
        offset += 1;
        let mut dest = vec![];
        let b = data[offset];
        offset += 1;
        match b {
            x @ 0x00...0xef => {
                dest = data[offset..offset + 1].to_vec();
                offset += 1;
            }
            0xfc => {
                dest = data[offset..offset + 2].to_vec();
                offset += 2;
            }
            0xfd => {
                dest = data[offset..offset + 4].to_vec();
                offset += 4;
            }
            0xfe => { /* todo */ }
            0xff => {
                dest = data[offset..offset + 32].to_vec();
                offset += 32;
            }
            _ => { /* todo */ }
        };
        let mut amount = data[offset..].to_vec();
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
    let tx = "0x250281ff784cb29a605b557c11a3e22520387c4377ded1734f56900d7f04946a0b70f3388472fb05e5189d652fa63b13f3aa290645f7bf6e10d43b6957d2e1038e213436c1ca465843ecb52f7319364ac2ad2c260992d1fcf7e12e9e11fa0e00a9ab8a0904000300ff8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48ed01";
    let data = hex::decode(tx).unwrap();
    let ot = OriginTransfer::decode(data).unwrap();
    assert_eq!(ot.amount, 123);
    assert_eq!(ot.index, 1);
}