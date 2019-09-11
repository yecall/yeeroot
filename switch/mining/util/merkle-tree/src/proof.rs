use crate::hash::Algorithm;
use serde_derive::{Deserialize, Serialize};

/// Merkle tree inclusion proof for data element, for which item = Leaf(Hash(Data Item)).
///
/// Lemma layout:
///
/// ```text
/// [ item h1x h2y h3z ... root ]
/// ```
///
/// Proof validation is positioned hash against lemma path to match root hash.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]

pub struct Proof<T: Eq + Clone + AsRef<[u8]>> {
   pub lemma: Vec<T>,
   pub path: Vec<bool>,
}

impl<T: Eq + Clone + AsRef<[u8]>> Proof<T> {
    /// Creates new MT inclusion proof
    pub fn new(hash: Vec<T>, path: Vec<bool>) -> Proof<T> {
        assert!(hash.len() > 2);
        assert_eq!(hash.len() - 2, path.len());
        Proof { lemma: hash, path }
    }

    /// Return proof target leaf
    pub fn item(&self) -> T {
        self.lemma.first().unwrap().clone()
    }

    /// Return tree root
    pub fn root(&self) -> T {
        self.lemma.last().unwrap().clone()
    }

    /// Verifies MT inclusion proof
    pub fn validate<A: Algorithm<T>>(&self) -> bool {
        let size = self.lemma.len();
        if size < 2 {
            return false;
        }

        let mut h = self.item();
        let mut a = A::default();

        for i in 1..size - 1 {
            a.reset();
            h = if self.path[i - 1] {
                a.node(h, self.lemma[i].clone())
            } else {
                a.node(self.lemma[i].clone(), h)
            };
        }

        h == self.root()
    }
}
