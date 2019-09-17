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

extern crate crypto;
use std::fmt;
use std::hash::Hasher;
use std::iter::FromIterator;
use yee_merkle::hash::{Algorithm, Hashable};
use yee_merkle::merkle::MerkleTree;
use crypto::sha2::Sha256;
use crypto::digest::Digest;
use primitives::H256;
use primitives::blake2_256;
use std::str::FromStr;
use primitives::hexdisplay::HexDisplay;


#[derive(Clone)]
struct CryptoYeeAlgorithm(Sha256);

impl CryptoYeeAlgorithm {
    fn new() -> CryptoYeeAlgorithm {
        CryptoYeeAlgorithm(Sha256::new())
    }
}

impl Default for CryptoYeeAlgorithm {
    fn default() -> CryptoYeeAlgorithm {
        CryptoYeeAlgorithm::new()
    }
}

impl Hasher for CryptoYeeAlgorithm {
    #[inline]
    fn write(&mut self, msg: &[u8]) {
        self.0.input(msg)
    }

    #[inline]
    fn finish(&self) -> u64 {
        unimplemented!()
    }
}

type CryptoSHA256Hash = [u8; 32];

impl Algorithm<CryptoSHA256Hash> for CryptoYeeAlgorithm {
    #[inline]
    fn hash(&mut self) -> CryptoSHA256Hash {
        let mut h = [0u8; 32];
        self.0.result(&mut h);

        // double sha256
        let mut c = Sha256::new();
        c.input(h.as_ref());
        c.result(&mut h);
        h
    }

    #[inline]
    fn reset(&mut self) {
        self.0.reset();
    }

    fn leaf(&mut self, leaf: CryptoSHA256Hash) -> CryptoSHA256Hash {
        leaf
    }

    fn node(&mut self, left: CryptoSHA256Hash, right: CryptoSHA256Hash) -> CryptoSHA256Hash {
        self.write(left.as_ref());
        self.write(right.as_ref());
        self.hash()
    }
}

struct HexSlice<'a>(&'a [u8]);

impl<'a> HexSlice<'a> {
    fn new<T>(data: &'a T) -> HexSlice<'a>
        where
            T: ?Sized + AsRef<[u8]> + 'a,
    {
        HexSlice(data.as_ref())
    }
}

/// reverse order
impl<'a> fmt::Display for HexSlice<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let len = self.0.len();
        for i in 0..len {
            let byte = self.0[len - 1 - i];
            write!(f, "{:x}{:x}", byte >> 4, byte & 0xf)?;
        }
        Ok(())
    }
}


fn main(){
    leaf_str();

//    let mut h1 = [0u8; 32];
//    let mut h2 = [0u8; 32];
//    let mut h3 = [0u8; 32];
//    h1[0] = 0x00;
//    h2[0] = 0x11;
//    h3[0] = 0x22;
//
//    let mut a = CryptoYeeAlgorithm::new();
//    let h11 = h1;
//    let h12 = h2;
//    let h13 = h3;
//    let h21 = a.node(h11, h12);
//    a.reset();
//    let h22 = a.node(h13, h13);
//    a.reset();
//    let h31 = a.node(h21, h22);
//    a.reset();
//
//    let xa = HexSlice::new(h21.as_ref());
//    let xb = HexSlice::new(h22.as_ref());
//    let xc = HexSlice::new(h31.as_ref());
//    println!("xa-{}",xa);
//    println!("xb-{}",xb);
//    println!("xc-{}",xc);
//
//
//    let t: MerkleTree<CryptoSHA256Hash, CryptoYeeAlgorithm> =
//        MerkleTree::from_iter(vec![h1, h2, h3]);
//
//
//
//    let root = t.root();
//
//    println!("root-{}",   HexSlice::new(t.root().as_ref()));
//
//    let proofa = t.gen_proof(0);
//
//    let proofb = t.gen_proof(1);
//
//    let proofc = t.gen_proof(2);
//
//    let f = proofa.validate::<CryptoYeeAlgorithm>();
//
//    println!("ff---{}",f)

}

fn leaf_str(){

//    let hash:H256 = blake2_256( "hehe".as_bytes()).into();
//    let xd: [u8;32] = hash.clone().into();
//   // println!("solve hash hash****{} ",hash);
//    println!("solve hash xd --{:?}", xd);
//
      let mut a = CryptoYeeAlgorithm::new();
//     let leaf =  a.node(xd,xd);
//    println!("leaf --{:?}", leaf);

//

//    "yee".hash(&mut a);
//    let h11 = a.hash();
//    println!("h11-{}", HexSlice::new(h11.as_ref()));
//    a.reset();
//
//    "yeeroot".hash(&mut a);
//    let h12 = a.hash();
//    println!("h12-{}", HexSlice::new(h12.as_ref()));
//    a.reset();
//
//    "yeeco".hash(&mut a);
//    let h13 = a.hash();
//    println!("h13-{}", HexSlice::new(h13.as_ref()));
//    a.reset();


//
//
//
//    let h21 = a.node(h11, h12);
//    a.reset();
//    let h22 = a.node(h13, h13);
//    a.reset();
//    let h31 = a.node(h21, h22);
//    a.reset();
//
//    let xa = HexSlice::new(h21.as_ref());
//    let xb = HexSlice::new(h22.as_ref());
//    let xc = HexSlice::new(h31.as_ref());
//    println!("h21-{}",xa);
//    println!("h22-{}",xb);
//    println!("h31-{}",xc);



    let ran = H256::random();
    let zero = H256::zero();
    let len = H256::len_bytes();
    let hex = "7a83ecb34b4f3e8ecd9440fa9fd57d0c87e499aee8a3662c33437a0ddbc6d9d1";

    //let x = i32::from_str(hex).unwrap();
    let ndkd:H256 = H256::from_str(hex).unwrap();
    let hexdis = HexDisplay::from(H256::as_fixed_bytes(&ndkd));

    let sttt =  ndkd.to_string();
    let borrowed_string ="0x".to_string();

    let disfmt = hexdis.to_string();
    let together = format!("{}{}", borrowed_string, disfmt);
    println!("work---H256::random()-{:?}",ran);
    println!("work---H256::ndkd()-{:?}",ndkd);
    println!("work---H256::sttt()-{:?}", together);

    //let together = format!("{}{}", borrowed_string, HexDisplay::from(H256::as_fixed_bytes(&ndkd)).to_string());


    let g1:H256= blake2_256( "hehe1".as_bytes()).into();
    let together1 = format!("{}{}", borrowed_string, HexDisplay::from(H256::as_fixed_bytes(&g1)).to_string());

    together1.hash(&mut a);
    let h1 = a.hash();
    println!("h1-{:?}", h1);
    a.reset();


    let g2:H256 = blake2_256( "hehe2".as_bytes()).into();
    let together2 = format!("{}{}", borrowed_string, HexDisplay::from(H256::as_fixed_bytes(&g2)).to_string());
    together2.hash(&mut a);
    let h2 = a.hash();
    println!("h2-{:?}", h2);
    a.reset();



    let g3:H256 = blake2_256( "hehe3".as_bytes()).into();
    let together3 = format!("{}{}", borrowed_string, HexDisplay::from(H256::as_fixed_bytes(&g3)).to_string());
    together3.hash(&mut a);
    let h3 = a.hash();
    println!("h3-{:?}", h3);
    a.reset();




    let t: MerkleTree<CryptoSHA256Hash, CryptoYeeAlgorithm> =
        MerkleTree::from_iter([together1,together2,together3].iter().map(|x|{
            a.reset();
            x.hash(&mut a);
            a.hash()
        }));


    let ln = t.leafs();
   // println!("ln--{}",ln);
    let hi = t.height();
   // println!("hi--{}",hi);
    //let data = t.deref();
    println!("t.data--{:?}",t.data  );


    for i in 0..t.leafs() {
        let p = t.gen_proof(i);
       p.validate::<CryptoYeeAlgorithm>();
    }



    let root = t.root();

    println!("root-{}",   HexSlice::new(t.root().as_ref()));

    let proofa = t.gen_proof(0);

    let proofb = t.gen_proof(1);

    let proofc = t.gen_proof(2);

    let f = proofa.validate::<CryptoYeeAlgorithm>();

   // println!("ff---{}",f);

    let root1 = proofa.root();
    let item =  proofc.item();
    let nn = proofa.lemma;

    println!("proofalemma---{:?}",nn);


  //  println!("root1---{:?}",root1);

    println!("item1---{:?}", item);


}

/// [](https://bitcoin.stackexchange.com/questions/5671/how-do-you-perform-double-sha-256-encoding)
#[test]
fn test_crypto_yee_leaf_hash() {
    let mut a = CryptoYeeAlgorithm::new();
    "hello".hash(&mut a);
    let h1 = a.hash();
    assert_eq!(
        format!("{}", HexSlice::new(h1.as_ref())),
        "503d8319a48348cdc610a582f7bf754b5833df65038606eb48510790dfc99595"
    );
}

/// [](http://chimera.labs.oreilly.com/books/1234000001802/ch07.html#merkle_trees)
#[test]
fn test_crypto_yee_node() {
    let mut h1 = [0u8; 32];
    let mut h2 = [0u8; 32];
    let mut h3 = [0u8; 32];
    h1[0] = 0x00;
    h2[0] = 0x11;
    h3[0] = 0x22;

    let mut a = CryptoYeeAlgorithm::new();
    let h11 = h1;
    let h12 = h2;
    let h13 = h3;
    let h21 = a.node(h11, h12);
    a.reset();
    let h22 = a.node(h13, h13);
    a.reset();
    let h31 = a.node(h21, h22);
    a.reset();

    assert_eq!(
        format!("{}", HexSlice::new(h21.as_ref())),
        "32650049a0418e4380db0af81788635d8b65424d397170b8499cdc28c4d27006"
    );
    assert_eq!(
        format!("{}", HexSlice::new(h22.as_ref())),
        "30861db96905c8dc8b99398ca1cd5bd5b84ac3264a4e1b3e65afa1bcee7540c4"
    );
    assert_eq!(
        format!("{}", HexSlice::new(h31.as_ref())),
        "d47780c084bad3830bcdaf6eace035e4c6cbf646d103795d22104fb105014ba3"
    );

    let t: MerkleTree<CryptoSHA256Hash, CryptoYeeAlgorithm> =
        MerkleTree::from_iter(vec![h1, h2, h3]);
    assert_eq!(
        format!("{}", HexSlice::new(t.root().as_ref())),
        "d47780c084bad3830bcdaf6eace035e4c6cbf646d103795d22104fb105014ba3"
    );
}