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

use {
	futures::{
		Future, IntoFuture,
		future,
	},
	log::warn,
	std::{
		fmt::Debug,
		marker::PhantomData,
		sync::Arc,
		time::{
			SystemTime, UNIX_EPOCH,
		},
	},
};
use {
	client::{
		ChainHead,
		blockchain::HeaderBackend,
	},
	consensus_common::{
		Environment, Proposer, BlockOrigin,  ForkChoiceStrategy, ImportBlock, BlockImport
	},
	inherents::InherentDataProviders,
	runtime_primitives::{
		generic::BlockId,
		traits::{Block, ProvideRuntimeApi, Zero, One, As, DigestItemFor, NumberFor, SimpleArithmetic, DigestFor, Digest, Header},
	},
};
use {
	super::{
		worker::to_common_error,
	},
};
use std::time::Duration;
use crate::{PowSeal, WorkProof, CompatibleDigestItem};
use parity_codec::{Decode, Encode};
use log::info;
use {
	pow_primitives::{YeePOWApi, DifficultyType},
};
use crate::pow::{check_proof, gen_extrinsic_proof};
use yee_sharding::ShardingDigestItem;
use primitives::H256;
use ansi_term::Colour;

#[derive(Clone)]
pub struct DefaultJob<B: Block, AuthorityId: Decode + Encode + Clone> {
	/// Hash for header with consensus post-digests (unknown WorkProof) applied
	/// The hash has 2 uses:
	/// 1. distinguish different job
	/// 2. build merkle tree of ProofMulti
	pub hash: B::Hash,
	/// The header, without consensus post-digests applied
	pub header: B::Header,
	/// Block's body
	pub body: Vec<B::Extrinsic>,
	/// Digest item
	pub digest_item: PowSeal<B, AuthorityId>,
}

impl<B: Block, AuthorityId: Decode + Encode + Clone> Job for DefaultJob<B, AuthorityId>{

	type Hash = B::Hash;
}

pub trait Job{
	type Hash;
}

pub trait JobManager: Send + Sync
{
	type Job : Job;

	/// get job with unknown proof
	fn get_job(&self) -> Box<dyn Future<Item=Self::Job, Error=consensus_common::Error> + Send>;

	/// submit job
	fn submit_job(&self, job: Self::Job) -> Box<dyn Future<Item=<Self::Job as Job>::Hash, Error=consensus_common::Error> + Send>;

}

pub struct DefaultJobManager<B, C, E, AuthorityId, I> {
	client: Arc<C>,
	env: Arc<E>,
	inherent_data_providers: InherentDataProviders,
	authority_id: AuthorityId,
	block_import: Arc<I>,
	phantom: PhantomData<B>,
}

impl<B, C, E, AuthorityId, I> DefaultJobManager<B, C, E, AuthorityId, I> where
	B: Block,
	C: ChainHead<B>,
	E: Environment<B> + 'static,
	<E as Environment<B>>::Proposer: Proposer<B>,
	<E as Environment<B>>::Error: Debug,
	<<E as Environment<B>>::Proposer as Proposer<B>>::Create: IntoFuture<Item=B>,
	<<<E as Environment<B>>::Proposer as Proposer<B>>::Create as IntoFuture>::Future: Send + 'static,
	AuthorityId: Decode + Encode + Clone,
	I: BlockImport<B, Error=consensus_common::Error> + Send + Sync + 'static,
{
	pub fn new(
		client: Arc<C>,
		env: Arc<E>,
		inherent_data_providers: InherentDataProviders,
		authority_id: AuthorityId,
		block_import: Arc<I>,
	) -> Self {
		Self {
			client,
			env,
			inherent_data_providers,
			authority_id,
			block_import,
			phantom: PhantomData,
		}
	}
}

impl<B, C, E, AuthorityId, I> JobManager for DefaultJobManager<B, C, E, AuthorityId, I>
	where B: Block,
	      DigestItemFor<B>: super::CompatibleDigestItem<B, AuthorityId> + ShardingDigestItem<u16>,
	      C: ChainHead<B> + Send + Sync + 'static,
	      C: HeaderBackend<B> + ProvideRuntimeApi,
	      <C as ProvideRuntimeApi>::Api: YeePOWApi<B>,
	      E: Environment<B> + 'static + Send + Sync,
	      <E as Environment<B>>::Proposer: Proposer<B>,
	      <E as Environment<B>>::Error: Debug,
	      <<E as Environment<B>>::Proposer as Proposer<B>>::Create: IntoFuture<Item=B>,
	      <<<E as Environment<B>>::Proposer as Proposer<B>>::Create as IntoFuture>::Future: Send + 'static,
	      AuthorityId: Decode + Encode + Clone + Send + Sync + 'static,
	      I: BlockImport<B, Error=consensus_common::Error> + Send + Sync + 'static,
          <B as Block>::Hash: From<H256> + Ord,
{
	type Job = DefaultJob<B, AuthorityId>;

	fn get_job(&self) -> Box<dyn Future<Item=Self::Job, Error=consensus_common::Error> + Send> {
		let get_data = || {
			let chain_head = self.client.best_block_header()
				.map_err(to_common_error)?;
			let proposer = self.env.init(&chain_head, &vec![])
				.map_err(to_common_error)?;
			let inherent_data = self.inherent_data_providers.create_inherent_data()
				.map_err(to_common_error)?;
			Ok((proposer, inherent_data))
		};
		let (proposer, inherent_data) = match get_data() {
			Ok((p, d)) => (p, d),
			Err(e) => {
				warn!("failed to get proposer {:?}", e);
				return Box::new(future::err(e));
			}
		};

		let client = self.client.clone();
		let authority_id = self.authority_id.clone();

		let build_job = move |block: B| {
			let (header, body) = block.deconstruct();
			let header_num = header.number().clone();
			let header_pre_hash = header.hash();
			let timestamp = timestamp_now()?;
			let difficulty = calc_difficulty(client, &header, timestamp)?;
			let authority_id = authority_id;
			let work_proof = WorkProof::Unknown;

			let pow_seal = PowSeal {
				authority_id,
				difficulty,
				timestamp,
				work_proof,
			};
			let mut header_with_pow_seal = header.clone();
			let item = <DigestItemFor<B> as CompatibleDigestItem<B, AuthorityId>>::pow_seal(pow_seal.clone());
			header_with_pow_seal.digest_mut().push(item);
			let hash = header_with_pow_seal.hash();

			info!("job {} @ {:?} difficulty {:#x}", header_num, header_pre_hash, difficulty);

			Ok(DefaultJob {
				hash,
				header,
				body,
				digest_item: pow_seal,
			})
		};

		Box::new(proposer.propose(inherent_data, Duration::from_secs(10)).into_future()
			.map_err(to_common_error).and_then(build_job))
	}

	fn submit_job(&self, job: Self::Job) -> Box<dyn Future<Item=<Self::Job as Job>::Hash, Error=consensus_common::Error> + Send>{

		let block_import = self.block_import.clone();

		let check_job = move |job: Self::Job| -> Result<<Self::Job as Job>::Hash, consensus_common::Error>{
			let number = &job.header.number().clone();
			let (post_digest, hash) = check_proof(&job.header, &job.digest_item)?;
			let extrinsic_proof = gen_extrinsic_proof::<B>(&job.header, job.body.as_slice());
			let import_block: ImportBlock<B> = ImportBlock {
				origin: BlockOrigin::Own,
				header: job.header,
				justification: None,
				proof: extrinsic_proof,
				post_digests: vec![post_digest],
				body: Some(job.body),
				finalized: false,
				auxiliary: Vec::new(),
				fork_choice: ForkChoiceStrategy::LongestChain,
			};
			block_import.import_block(import_block, Default::default())?;
			info!("{} @ {} {:?}", Colour::Green.bold().paint("Block Mined"), number, hash);
			Ok(hash)
		};

		Box::new(check_job(job).into_future())

	}
}

fn timestamp_now() -> Result<u64, consensus_common::Error> {
	Ok(SystemTime::now().duration_since(UNIX_EPOCH)
		.map_err(to_common_error)?.as_millis() as u64)
}

fn calc_difficulty<B, C, AuthorityId>(
	client: Arc<C>, header: &<B as Block>::Header, timestamp: u64,
) -> Result<DifficultyType, consensus_common::Error> where
	B: Block,
	NumberFor<B>: SimpleArithmetic,
	DigestFor<B>: Digest,
	DigestItemFor<B>: super::CompatibleDigestItem<B, AuthorityId>,
	C: HeaderBackend<B> + ProvideRuntimeApi,
	<C as ProvideRuntimeApi>::Api: YeePOWApi<B>,
	AuthorityId: Encode + Decode + Clone,
{
	let curr_block_id = BlockId::hash(*header.parent_hash());
	let api = client.runtime_api();
	let genesis_difficulty = api.genesis_difficulty(&curr_block_id)
		.map_err(to_common_error)?;
	let adj = api.difficulty_adj(&curr_block_id)
		.map_err(to_common_error)?;
	let curr_header = client.header(curr_block_id)
		.expect("parent block must exist for sealer; qed")
		.expect("parent block must exist for sealer; qed");

	// not on adjustment, reuse parent difficulty
	if *header.number() % adj != Zero::zero() {
		let curr_difficulty = curr_header.digest().logs().iter().rev()
			.filter_map(CompatibleDigestItem::as_pow_seal).next()
			.and_then(|seal| Some(seal.difficulty))
			.unwrap_or(genesis_difficulty);
		return Ok(curr_difficulty);
	}

	let mut curr_header = curr_header;
	let mut curr_seal = curr_header.digest().logs().iter().rev()
		.filter_map(CompatibleDigestItem::as_pow_seal).next()
		.expect("Seal must exist when adjustment comes; qed");
	let curr_difficulty = curr_seal.difficulty;
	let (last_num, last_time) = loop {
		let prev_header = client.header(BlockId::hash(*curr_header.parent_hash()))
			.expect("parent block must exist for sealer; qed")
			.expect("parent block must exist for sealer; qed");
		assert!(*prev_header.number() + One::one() == *curr_header.number());
		let prev_seal = prev_header.digest().logs().iter().rev()
			.filter_map(CompatibleDigestItem::as_pow_seal).next();
		if *prev_header.number() % adj == Zero::zero() {
			break (curr_header.number(), curr_seal.timestamp);
		}
		if let Some(prev_seal) = prev_seal {
			curr_header = prev_header;
			curr_seal = prev_seal;
		} else {
			break (curr_header.number(), curr_seal.timestamp);
		}
	};

	let target_block_time = api.target_block_time(&curr_block_id)
		.map_err(to_common_error)?;
	let block_gap = As::<u64>::as_(*header.number() - *last_num);
	let time_gap = timestamp - last_time;
	let expected_gap = target_block_time * 1000 * block_gap;
	let new_difficulty = (curr_difficulty / expected_gap) * time_gap;
	info!("difficulty adjustment: gap {} time {}", block_gap, time_gap);
	info!("    new difficulty {:#x}", new_difficulty);

	Ok(new_difficulty)
}
