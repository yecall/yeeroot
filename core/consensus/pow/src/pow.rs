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

//! POW work proof used in block header digest
use std::sync::Arc;
use std::fmt::Debug;
use std::collections::hash_map::HashMap;
use runtime_primitives::{
    codec::{
        Decode, Encode,
    },
    traits::{Block, DigestItemFor, DigestFor, Digest, Header, Hash as HashT, BlakeTwo256,
             Zero, As, SimpleArithmetic, NumberFor},
    generic::BlockId,
    Proof as ExtrinsicProof,
};
use client::blockchain::HeaderBackend;
use pow_primitives::{PowTarget, YeePOWApi};
use crate::CompatibleDigestItem;
use yee_sharding::ShardingDigestItem;
use log::{debug, info};
use std::marker::PhantomData;
use std::hash::Hasher;
use merkle_light::hash::Algorithm;
use merkle_light::proof::Proof;
use merkle_light::merkle::MerkleTree;
use yee_runtime::{Call, BalancesCall, UncheckedExtrinsic};
use yee_sharding_primitives::utils::shard_num_for;
use primitives::{Blake2Hasher, H256};
use hash_db::Hasher as BlakeHasher;
use std::iter::FromIterator;
use yee_merkle::{ProofHash, ProofAlgorithm, MultiLayerProof};
use ansi_term::Colour;
use yee_context::Context;

/// Max length in bytes for pow extra data
pub const MAX_EXTRA_DATA_LENGTH: usize = 32;

/// POW consensus seal
#[derive(Clone, Debug, Decode, Encode)]
pub struct PowSeal<B: Block, AuthorityId: Decode + Encode + Clone> {
    pub authority_id: AuthorityId,
    pub pow_target: PowTarget,
    pub timestamp: u64,
    pub work_proof: WorkProof<B>,
    pub relay_proof: H256,
}

/// POW proof used in block header
#[derive(Clone, Debug)]
#[derive(Decode, Encode)]
pub enum WorkProof<B: Block> {
    #[codec(index = "0")]
    Unknown,
    #[codec(index = "1")]
    Nonce(ProofNonce),
    #[codec(index = "2")]
    Multi(ProofMulti<B>),
}

/// Classical pow proof with extra data entropy and 64b nonce
#[derive(Clone, Debug)]
#[derive(Decode, Encode)]
pub struct ProofNonce {
    /// Extra Data used to encode miner info AND more entropy
    pub extra_data: Vec<u8>,
    /// POW block nonce
    pub nonce: u64,
}

impl ProofNonce {
    pub fn get_with_prefix_len(prefix: &str, extra_bytes: usize, nonce: u64) -> Self {
        let len = prefix.len() + extra_bytes;
        assert!(len <= MAX_EXTRA_DATA_LENGTH);

        let mut extra_data = prefix.as_bytes().to_vec();
        extra_data.resize_with(len, Default::default);
        Self {
            extra_data,
            nonce,
        }
    }
}

/// Multi-Mining pow proof with header-trie spv proof
#[derive(Clone, Debug)]
#[derive(Decode, Encode)]
pub struct ProofMulti<B: Block> {
    /// Extra Data used to encode miner info AND more entropy
    pub extra_data: Vec<u8>,
    /// merkle root of multi-mining headers
    pub merkle_root: B::Hash,
    /// POW block nonce
    pub nonce: u64,
    /// merkle proof
    pub merkle_proof: Vec<B::Hash>,
}

/// Check proof
///
/// Returns (post_digest, hash)
pub fn check_work_proof<B, AuthorityId>(header: &B::Header, seal: &PowSeal<B, AuthorityId>) -> Result<(DigestItemFor<B>, B::Hash), String> where
    B: Block,
    AuthorityId: Decode + Encode + Clone,
    DigestFor<B>: Digest,
    DigestItemFor<B>: CompatibleDigestItem<B, AuthorityId> + ShardingDigestItem<u16>,
{
    match seal.work_proof{
        WorkProof::Unknown => Err(format!("invalid work proof")),
        WorkProof::Nonce(ref proof_nonce) => {
            if proof_nonce.extra_data.len() > MAX_EXTRA_DATA_LENGTH {
                return Err(format!("extra data too long"));
            }

            let mut work_header = header.clone();
            let seal_owned : PowSeal<B, AuthorityId> = seal.to_owned();
            let item = <DigestItemFor<B> as CompatibleDigestItem<B, AuthorityId>>::pow_seal(seal_owned);
            work_header.digest_mut().push(item);

            let hash = work_header.hash();

            let proof_pow_target = PowTarget::from(hash.as_ref());

            if proof_pow_target > seal.pow_target {
                return Err(format!("Nonce proof: pow target not satisified, need {}, got {}", seal.pow_target, proof_pow_target));
            }

            let post_digest = work_header.digest_mut().pop().expect("must exist");

            Ok((post_digest, hash))
        },
        WorkProof::Multi(ref proof_multi) => {

            let shard_info = header.digest().logs().iter().rev()
                .filter_map(ShardingDigestItem::as_sharding_info)
                .next();

            let (shard_num, shard_count) = shard_info.expect("must exist");
            let (shard_num, shard_count) = (shard_num as u16, shard_count as u16);
            debug!("Check multi proof: shard_num: {}, shard_count: {}", shard_num, shard_count);

            //get pre hash
            let pow_seal = PowSeal {
                authority_id: seal.authority_id.clone(),
                pow_target: seal.pow_target,
                timestamp: seal.timestamp,
                work_proof: WorkProof::Unknown,
                relay_proof: seal.relay_proof.clone(),
            };
            let mut header_with_pow_seal = header.clone();
            let item = <DigestItemFor<B> as CompatibleDigestItem<B, AuthorityId>>::pow_seal(pow_seal.clone());
            header_with_pow_seal.digest_mut().push(item);
            let pre_hash = header_with_pow_seal.hash();

            let merkle_proof_item_count = 1u16 << proof_multi.merkle_proof.len() as u16;

            // merkle_proof_item_count should match shard_count (normal or scaling)
            let (num, count) = if merkle_proof_item_count == shard_count {
                (shard_num, shard_count)
            } else if merkle_proof_item_count == shard_count * 2u16 {
                (shard_num, shard_count * 2u16)
            } else {
                return Err(format!("Invalid merkle proof item count"));
            };

            // merkle proof validate
            let compact_proof = CompactMerkleProof::<<B::Header as Header>::Hashing> {
                proof: proof_multi.merkle_proof.clone(),
                item: pre_hash,
                root: proof_multi.merkle_root.clone(),
                num: num,
                count: count,
            };
            let original_proof = parse_original(compact_proof)?;

            debug!("Check multi proof: original_proof: {:?}", original_proof);

            let valid = original_proof.proof.validate::<MiningAlgorithm<<B::Header as Header>::Hashing>>();

            if !valid {
                return Err(format!("Invalid merkle proof"));
            }

            //diff validate
            let source = (proof_multi.merkle_root.clone(), proof_multi.extra_data.clone(), proof_multi.nonce);
            let source_hash = <B::Header as Header>::Hashing::hash_of(&source);
            let source_pow_target = PowTarget::from(source_hash.as_ref());

            if source_pow_target > seal.pow_target {
                return Err(format!("Multi proof: pow target not satisfied, need {}, got {}", seal.pow_target, source_pow_target));
            }

            //make hash
            let mut work_header = header.clone();
            let seal_owned : PowSeal<B, AuthorityId> = seal.to_owned();
            let item = <DigestItemFor<B> as CompatibleDigestItem<B, AuthorityId>>::pow_seal(seal_owned);
            work_header.digest_mut().push(item);

            let hash = work_header.hash();

            let post_digest = work_header.digest_mut().pop().expect("must exist");

            Ok((post_digest, hash))
        }
    }
}

fn to_common_error<E: Debug>(e: E) -> consensus_common::Error {
    consensus_common::ErrorKind::ClientImport(format!("{:?}", e)).into()
}

/// calculate pow target
pub fn calc_pow_target<B, C, AuthorityId>(client: Arc<C>, header: &<B as Block>::Header, timestamp: u64, context: &Context<B>)
    -> Result<PowTarget, consensus_common::Error> where
    B: Block,
    NumberFor<B>: SimpleArithmetic,
    DigestFor<B>: Digest,
    DigestItemFor<B>: super::CompatibleDigestItem<B, AuthorityId>,
    C: HeaderBackend<B>,
    AuthorityId: Encode + Decode + Clone,
{
    let next_num = *header.number();
    let curr_block_id = BlockId::hash(*header.parent_hash());
    let genesis_pow_target = context.genesis_pow_target;
    let adj = context.genesis_pow_target_adj;
    let curr_header = client.header(curr_block_id)
        .expect("parent block must exist for sealer; qed")
        .expect("parent block must exist for sealer; qed");
    let one = <NumberFor<B> as As<u64>>::sa(1u64);
    // not on adjustment, reuse parent pow target
    if next_num == one {
        return Ok(genesis_pow_target)
    } else if (next_num - one) % adj != Zero::zero() {
        let curr_pow_target = curr_header.digest().logs().iter().rev()
            .filter_map(CompatibleDigestItem::as_pow_seal).next()
            .and_then(|seal| Some(seal.pow_target))
            .unwrap_or(genesis_pow_target);
        return Ok(curr_pow_target);
    }

    let curr_seal = curr_header.digest().logs().iter().rev()
        .filter_map(CompatibleDigestItem::as_pow_seal).next()
        .expect("Seal must exist when adjustment comes; qed");
    let curr_pow_target = curr_seal.pow_target;

    let (block_gap, last_time) = {
        let id = BlockId::<B>::number(next_num - adj);
        let ancestor_header = client.header(id)
            .expect("parent block must exist for sealer; qed")
            .expect("parent block must exist for sealer; qed");
        let ancestor_seal = ancestor_header.digest().logs().iter().rev()
            .filter_map(CompatibleDigestItem::as_pow_seal).next();
        match ancestor_seal {
            Some(seal) => {
                (adj.as_(), seal.timestamp)
            }
            None => {
                panic!("can't get PowSeal in pre-block's header")
            }
        }
    };

    let target_block_time = context.genesis_target_block_time;
    let time_gap = timestamp - last_time;
    let expected_gap = target_block_time * 1000 * block_gap;
    let new_pow_target = (curr_pow_target / expected_gap) * time_gap;
    info!("pow target adjustment: gap: {}, time: {}", block_gap, time_gap);
    info!("old pow target: {:#x}, new pow target: {:#x}",curr_pow_target, new_pow_target);

    Ok(new_pow_target)
}

/// Gen extrinsic proof for foreign chain.
pub fn gen_extrinsic_proof<B>(header: &B::Header, body: &[B::Extrinsic]) -> (H256, ExtrinsicProof)
    where
        B: Block,
        <<<B as Block>::Header as Header>::Digest as Digest>::Item: yee_sharding::ShardingDigestItem<u16>,
        <B as Block>::Hash: From<H256> + Ord,
{
    let shard_info = header.digest().logs().iter().rev()
        .filter_map(ShardingDigestItem::as_sharding_info)
        .next();
    let (shard_num, shard_count) = shard_info.expect("must exist");
    let (shard_num, shard_count) = (shard_num as u16, shard_count as u16);

    let mut extrinsic_shard: HashMap<u16, Vec<H256>> = HashMap::new();
    for extrinsic in body {
        let bytes = extrinsic.encode();
        let mut bytes = bytes.as_slice();
        if let Some(ex) = Decode::decode(&mut bytes) {
            let ex: UncheckedExtrinsic = ex;
            if ex.signature.is_some() {
                let hash = Blake2Hasher::hash(&mut bytes);
                if let Call::Balances(BalancesCall::transfer(to, _)) = ex.function {
                    if let Some(num) = shard_num_for(&to, shard_count) {
                        if num != shard_num {
                            if let Some(list) = extrinsic_shard.get_mut(&num) {
                                list.push(hash);
                            } else {
                                extrinsic_shard.insert(num, vec![hash]);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut layer1_merkles = Vec::new();
    let mut layer2_leaves = vec![];
    for i in 0..shard_count {
        if extrinsic_shard.contains_key(&i){
            let exs = extrinsic_shard.get(&i).unwrap();
            let tree = MerkleTree::from_iter((*exs).clone());
            layer2_leaves.push(tree.root());
            layer1_merkles.push((i, Some(tree)));
        } else {
            let hash: H256 = Default::default();
            layer2_leaves.push(hash);
            layer1_merkles.push((i, None));
        }
    }
    let layer2_tree = MerkleTree::<ProofHash<BlakeTwo256>, ProofAlgorithm<BlakeTwo256>>::new(layer2_leaves);
    let layer2_root = layer2_tree.root();
    let multi_proof = MultiLayerProof::new_with_layer2(layer2_tree, layer1_merkles, );
    debug!("{} height:{}, proof: {:?}", Colour::White.bold().paint("Gen proof"), header.number(), &multi_proof);
    (layer2_root, multi_proof.into_bytes())
}

#[derive(Clone, Debug)]
pub struct MiningAlgorithm<H: HashT>(Vec<u8>, PhantomData<H>);

impl<H: HashT> MiningAlgorithm<H> {
    fn new() -> MiningAlgorithm<H> {
        MiningAlgorithm(Vec::with_capacity(32), PhantomData)
    }
}

impl<H: HashT> Default for MiningAlgorithm<H> {
    fn default() -> MiningAlgorithm<H> {
        MiningAlgorithm::new()
    }
}

impl<H: HashT> Hasher for MiningAlgorithm<H> {
    #[inline]
    fn write(&mut self, msg: &[u8]) {
        self.0.extend_from_slice(msg)
    }

    #[inline]
    fn finish(&self) -> u64 {
        unimplemented!()
    }
}

pub type MiningHash<H> = <H as HashT>::Output;

impl<H: HashT> Algorithm<MiningHash<H>> for MiningAlgorithm<H> where H::Output: Encode + Decode {
    #[inline]
    fn hash(&mut self) -> MiningHash<H> {
        H::hash(&self.0)
    }

    #[inline]
    fn reset(&mut self) {
        self.0.truncate(0);
    }

    fn leaf(&mut self, leaf: MiningHash<H>) -> MiningHash<H> {
        leaf
    }

    fn node(&mut self, left: MiningHash<H>, right: MiningHash<H>) -> MiningHash<H> {
        self.write(left.as_ref());
        self.write(right.as_ref());
        self.hash()
    }
}

#[derive(Debug, PartialEq)]
pub struct OriginalMerkleProof<H: HashT> where H::Output: Encode + Decode {
    pub proof: Proof<MiningHash<H>>,
    pub num: u16,
    pub count: u16,
}

#[derive(Debug, PartialEq)]
pub struct CompactMerkleProof<H: HashT> {
    pub proof: Vec<H::Output>,
    pub item: H::Output,
    pub root: H::Output,
    pub num: u16,
    pub count: u16,
}

impl<H: HashT> From<OriginalMerkleProof<H>> for CompactMerkleProof<H> where H::Output: Encode + Decode {
    fn from(omp: OriginalMerkleProof<H>) -> Self{
        let lemma = omp.proof.lemma();
        let len = lemma.len();

        let item = omp.proof.item();
        let root = omp.proof.root();

        //exclude first(leaf to proof) and last(root)
        let mut proof = Vec::new();
        for i in 1..(len-1){
            let node = lemma[i];
            proof.push(node)
        }

        Self {
            proof,
            item,
            root,
            num: omp.num,
            count: omp.count,
        }
    }
}

fn parse_original<H>(cmp: CompactMerkleProof<H>) -> Result<OriginalMerkleProof<H>, String> where
    H: HashT,
    H::Output: Encode + Decode,
{
    let num = cmp.num;
    let count = cmp.count;
    if num >= count{
        return Err(format!("Invalid num: {}", num));
    }

    let height = log2(count);
    if pow2(height) != count {
        return Err(format!("Invalid count: {}", count));
    }

    let compact_proof = cmp.proof;
    let expect_proof_size = height as usize;

    if compact_proof.len() != expect_proof_size {
        return Err(format!("Invalid proof size: {}, expected: {}", compact_proof.len(), expect_proof_size));
    }

    let mut lemma = Vec::new();
    lemma.push(cmp.item);
    lemma.extend(compact_proof);
    lemma.push(cmp.root);

    let mut path = Vec::new();
    let mut tmp_num = num;
    for _ in 0..height{
        let path_item = tmp_num % 2 == 0 ;
        path.push(path_item);
        tmp_num = tmp_num / 2;
    }

    Ok(OriginalMerkleProof{
        proof: Proof::new(lemma, path),
        num: num,
        count: count,
    })
}

fn log2(n: u16) -> u16 {
    let mut s = n;
    let mut i = 0;
    while s > 0 {
        s = s >> 1;
        i = i + 1;
    }
    i - 1
}

fn pow2(n: u16) -> u16{
    1u16 << n
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use yee_runtime::Block;
    use runtime_primitives::traits::{Hash as HashT, BlakeTwo256};
    use primitives::H256;
    use merkle_light::merkle::MerkleTree;
    use merkle_light::proof::Proof;
    use std::iter::FromIterator;

    #[test]
    fn proof_nonce_encode() {
        let tests = vec![
            ("Test Extra", 0x10001, hex!("01 28 54657374204578747261 0100 0100 0000 0000").to_vec()),
            ("YeeRoot", 0xABCD000098760000, hex!("01 1C 596565526f6f74 0000 7698 0000 CDAB").to_vec()),
            ("Yee Root", 0xABCD000098765432, hex!("01 20 59656520526f6f74 3254 7698 0000 CDAB").to_vec()),
        ];

        for (extra, nonce, expected) in tests {
            let proof = WorkProof::<Block>::Nonce(ProofNonce { extra_data: extra.as_bytes().to_vec(), nonce });
            assert_eq!(proof.encode(), expected);
        }
    }

    #[test]
    fn test_merkle_root() {
        let shard0_header_hash: H256 = [1u8; 32].into();
        let shard1_header_hash: H256 = [2u8; 32].into();
        let shard2_header_hash: H256 = [3u8; 32].into();
        let shard3_header_hash: H256 = [4u8; 32].into();

        let mut a = MiningAlgorithm::<BlakeTwo256>::new();

        let hash10 = a.node(shard0_header_hash, shard1_header_hash);

        a.reset();
        let hash11 = a.node(shard2_header_hash, shard3_header_hash);

        a.reset();
        let root = a.node(hash10, hash11);

        let t: MerkleTree<MiningHash<BlakeTwo256>, MiningAlgorithm<BlakeTwo256>> =
            MerkleTree::from_iter(vec![shard0_header_hash, shard1_header_hash, shard2_header_hash, shard3_header_hash]);

        let a = t.root();

        assert_eq!(root, a);
    }

    #[test]
    fn test_merkle_proof() {
        let shard0_header_hash: H256 = [1u8; 32].into();
        let shard1_header_hash: H256 = [2u8; 32].into();
        let shard2_header_hash: H256 = [3u8; 32].into();
        let shard3_header_hash: H256 = [4u8; 32].into();

        let mut a = MiningAlgorithm::<BlakeTwo256>::new();

        let hash10 = a.node(shard0_header_hash, shard1_header_hash);

        a.reset();
        let hash11 = a.node(shard2_header_hash, shard3_header_hash);

        a.reset();
        let root = a.node(hash10, hash11);

        let t: MerkleTree<MiningHash<BlakeTwo256>, MiningAlgorithm<BlakeTwo256>> =
            MerkleTree::from_iter(vec![shard0_header_hash, shard1_header_hash, shard2_header_hash, shard3_header_hash]);

        assert_eq!(Proof::new(vec![shard0_header_hash, shard1_header_hash, hash11, root], vec![true, true]), t.gen_proof(0));

        assert_eq!(Proof::new(vec![shard1_header_hash, shard0_header_hash, hash11, root], vec![false, true]), t.gen_proof(1));

        assert_eq!(Proof::new(vec![shard2_header_hash, shard3_header_hash, hash10, root], vec![true, false]), t.gen_proof(2));

        assert_eq!(Proof::new(vec![shard3_header_hash, shard2_header_hash, hash10, root], vec![false, false]), t.gen_proof(3));
    }

    #[test]
    fn test_merkle_proof_to_compact() {
        let shard0_header_hash: H256 = [1u8; 32].into();
        let shard1_header_hash: H256 = [2u8; 32].into();
        let shard2_header_hash: H256 = [3u8; 32].into();
        let shard3_header_hash: H256 = [4u8; 32].into();

        let mut a = MiningAlgorithm::<BlakeTwo256>::new();

        let hash10 = a.node(shard0_header_hash, shard1_header_hash);

        a.reset();
        let hash11 = a.node(shard2_header_hash, shard3_header_hash);

        a.reset();
        let root = a.node(hash10, hash11);

        let t: MerkleTree<MiningHash<BlakeTwo256>, MiningAlgorithm<BlakeTwo256>> =
            MerkleTree::from_iter(vec![shard0_header_hash, shard1_header_hash, shard2_header_hash, shard3_header_hash]);

        let proof = t.gen_proof(1);

        let ori_proof = OriginalMerkleProof::<BlakeTwo256>{
            proof,
            num: 1,
            count: 4,
        };

        let compact_proof : CompactMerkleProof<BlakeTwo256> = ori_proof.into();

        let mut proof = Vec::new();
        proof.push(shard0_header_hash);
        proof.push(hash11);
        let compact_proof2 = CompactMerkleProof::<BlakeTwo256>{
            proof: proof,
            item: shard1_header_hash,
            root: root,
            num: 1,
            count: 4,
        };

        assert_eq!(compact_proof, compact_proof2);
    }

    #[test]
    fn test_parse_original() {
        let shard0_header_hash: H256 = [1u8; 32].into();
        let shard1_header_hash: H256 = [2u8; 32].into();
        let shard2_header_hash: H256 = [3u8; 32].into();
        let shard3_header_hash: H256 = [4u8; 32].into();

        let mut a = MiningAlgorithm::<BlakeTwo256>::new();

        let hash10 = a.node(shard0_header_hash, shard1_header_hash);

        a.reset();
        let hash11 = a.node(shard2_header_hash, shard3_header_hash);

        a.reset();
        let root = a.node(hash10, hash11);

        let t: MerkleTree<MiningHash<BlakeTwo256>, MiningAlgorithm<BlakeTwo256>> =
            MerkleTree::from_iter(vec![shard0_header_hash, shard1_header_hash, shard2_header_hash, shard3_header_hash]);

        //1
        let proof = t.gen_proof(1);

        let ori_proof = OriginalMerkleProof::<BlakeTwo256>{
            proof,
            num: 1,
            count: 4,
        };

        let mut proof = Vec::new();
        proof.push(shard0_header_hash);
        proof.push(hash11);
        let compact_proof = CompactMerkleProof::<BlakeTwo256>{
            proof: proof,
            item: shard1_header_hash,
            root: root,
            num: 1,
            count: 4,
        };
        let ori_proof2 = parse_original(compact_proof).expect("qed");

        assert_eq!(ori_proof, ori_proof2);

        //2
        let proof = t.gen_proof(2);

        let ori_proof = OriginalMerkleProof::<BlakeTwo256>{
            proof,
            num: 2,
            count: 4,
        };

        let mut proof = Vec::new();
        proof.push(shard3_header_hash);
        proof.push(hash10);
        let compact_proof = CompactMerkleProof::<BlakeTwo256>{
            proof: proof,
            item: shard2_header_hash,
            root: root,
            num: 2,
            count: 4,
        };
        let ori_proof2 = parse_original(compact_proof).expect("qed");

        assert_eq!(ori_proof, ori_proof2);

    }
}
