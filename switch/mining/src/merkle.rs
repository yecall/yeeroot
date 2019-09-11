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


#[derive(Clone)]
pub struct CryptoYeeAlgorithm(Sha256);

impl CryptoYeeAlgorithm {
   pub fn new() -> CryptoYeeAlgorithm {
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
        //unimplemented!()
        0
    }
}

pub type CryptoSHA256Hash = [u8; 32];

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

pub struct HexSlice<'a>(&'a [u8]);

impl<'a> HexSlice<'a> {
   pub fn new<T>(data: &'a T) -> HexSlice<'a>
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


fn leaf_str(){

//    let hash:H256 = blake2_256( "YeeRoot".as_bytes()).into();
//    let xd: [u8;32] = hash.clone().into();
//    println!("solve hash hash****{} ",hash);
//    println!("solve hash xd --{:?}", xd);
//
    let mut a = CryptoYeeAlgorithm::new();
//
//    "yee".hash(&mut a);
//    //let h11 = a.hash();
//
//    let h11 = xd;
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


    let t: MerkleTree<CryptoSHA256Hash, CryptoYeeAlgorithm> =
        MerkleTree::from_iter(vec!["yee", "yeeroot", "yeeco"].iter().map(|x|{
            a.reset();
            x.hash(&mut a);
            a.hash()
        }));


    let ln = t.leafs();
    println!("ln--{}",ln);
    let hi = t.height();
    println!("hi--{}",hi);
    //let data = t.deref();



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

    println!("ff---{}",f)


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