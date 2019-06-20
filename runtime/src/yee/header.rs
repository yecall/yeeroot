//

use runtime_primitives::{
    codec::{
        Codec, Decode, Encode, EncodeAsRef,
        HasCompact,
        Input, Output,
    },
    generic::{
        Digest,
    },
    traits::{
        self, Member,
        DigestItem as DigestItemT, Hash as HashT,
        MaybeDisplay, MaybeSerializeDebug, MaybeSerializeDebugButNotDeserialize,
        SimpleArithmetic, SimpleBitOps,
    },
};

#[cfg(feature = "std")]
use serde::Serialize;

use super::WorkProof;

#[cfg(feature = "std")]
pub fn serialize_number<S, T: Copy + Into<u128>>(val: &T, s: S) -> Result<S::Ok, S::Error> where S: ::serde::Serializer {
    use primitives::uint::U256;
    let v: u128 = (*val).into();
    let lower = U256::from(v as u64);
    let upper = U256::from(v.rotate_left(64) as u64) << 64;
    ::serde::Serialize::serialize(&(upper + lower), s)
}

#[derive(PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(Debug, Serialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "std", serde(deny_unknown_fields))]
pub struct Header<Number: Copy + Into<u128>, Hash: HashT, DigestItem, Difficulty: Copy, AccountId: Clone> {
    /// The parent hash.
    pub parent_hash: <Hash as HashT>::Output,
    /// The block number.
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_number"))]
    pub number: Number,
    /// The state trie merkle root
    pub state_root: <Hash as HashT>::Output,
    /// The merkle root of the extrinsics.
    pub extrinsics_root: <Hash as HashT>::Output,
    /// A chain-specific digest of data useful for light clients or referencing auxiliary data.
    pub digest: Digest<DigestItem>,
    /// POW difficulty config
    pub difficulty: Difficulty,
    /// Block reward receiver
    pub coinbase: AccountId,
    /// POW work proof
    pub work_proof: Option<WorkProof>,
}

impl<Number, Hash, DigestItem, Difficulty, AccountId> Decode for Header<Number, Hash, DigestItem, Difficulty, AccountId> where
    Number: HasCompact + Copy + Into<u128>,
    Hash: HashT,
    Hash::Output: Decode,
    DigestItem: DigestItemT + Decode,
    Difficulty: Decode + Copy,
    AccountId: Decode + Clone,
{
    fn decode<I: Input>(input: &mut I) -> Option<Self> {
        Some(Header {
            parent_hash: Decode::decode(input)?,
            number: <<Number as HasCompact>::Type>::decode(input)?.into(),
            state_root: Decode::decode(input)?,
            extrinsics_root: Decode::decode(input)?,
            digest: Decode::decode(input)?,
            difficulty: Decode::decode(input)?,
            coinbase: Decode::decode(input)?,
            work_proof: Decode::decode(input)?,
        })
    }
}

impl<Number, Hash, DigestItem, Difficulty, AccountId> Encode for Header<Number, Hash, DigestItem, Difficulty, AccountId> where
    Number: HasCompact + Copy + Into<u128>,
    Hash: HashT,
    Hash::Output: Encode,
    DigestItem: DigestItemT + Encode,
    Difficulty: Encode + Copy,
    AccountId: Encode + Clone,
{
    fn encode_to<T: Output>(&self, dest: &mut T) {
        dest.push(&self.parent_hash);
        dest.push(&<<<Number as HasCompact>::Type as EncodeAsRef<_>>::RefType>::from(&self.number));
        dest.push(&self.state_root);
        dest.push(&self.extrinsics_root);
        dest.push(&self.digest);
        dest.push(&self.difficulty);
        dest.push(&self.coinbase);
        dest.push(&self.work_proof);
    }
}

impl<Number, Hash, DigestItem, Difficulty, AccountId> traits::Header for Header<Number, Hash, DigestItem, Difficulty, AccountId> where
    Number: Member + MaybeSerializeDebug + ::rstd::hash::Hash + MaybeDisplay + SimpleArithmetic + Codec + Copy + Into<u128>,
    Hash: HashT,
    DigestItem: DigestItemT<Hash = Hash::Output> + Codec,
    Difficulty: Member + MaybeSerializeDebug + Codec + Copy + Default,
    AccountId: Member + MaybeSerializeDebug + Codec + Default,
    Hash::Output: Default + ::rstd::hash::Hash + Copy + Member + MaybeSerializeDebugButNotDeserialize + MaybeDisplay + SimpleBitOps + Codec,
{
    type Number = Number;
    type Hash = <Hash as HashT>::Output;
    type Hashing = Hash;
    type Digest = Digest<DigestItem>;

    fn number(&self) -> &Self::Number { &self.number }
    fn set_number(&mut self, num: Self::Number) { self.number = num }

    fn extrinsics_root(&self) -> &Self::Hash { &self.extrinsics_root }
    fn set_extrinsics_root(&mut self, root: Self::Hash) { self.extrinsics_root = root }

    fn state_root(&self) -> &Self::Hash { &self.state_root }
    fn set_state_root(&mut self, root: Self::Hash) { self.state_root = root }

    fn parent_hash(&self) -> &Self::Hash { &self.parent_hash }
    fn set_parent_hash(&mut self, hash: Self::Hash) { self.parent_hash = hash }

    fn digest(&self) -> &Self::Digest { &self.digest }
    fn digest_mut(&mut self) -> &mut Self::Digest { &mut self.digest }
    fn set_digest(&mut self, digest: Self::Digest) { self.digest = digest }

    fn new(
        number: Self::Number,
        extrinsics_root: Self::Hash,
        state_root: Self::Hash,
        parent_hash: Self::Hash,
        digest: Self::Digest
    ) -> Self {
        Header {
            number,
            extrinsics_root,
            state_root,
            parent_hash,
            digest,
            difficulty: Default::default(),
            coinbase: Default::default(),
            work_proof: None,
        }
    }
}
