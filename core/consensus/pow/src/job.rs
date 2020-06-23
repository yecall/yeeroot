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
		traits::{Block, ProvideRuntimeApi, DigestItemFor, NumberFor, Digest, Header},
	},
};
use {
	super::{
		worker::to_common_error,
	},
};
use std::time::Duration;
use crate::{PowSeal, WorkProof, CompatibleDigestItem, ShardExtra};
use parity_codec::{Decode, Encode};
use log::info;
use {
	pow_primitives::YeePOWApi,
};
use crate::pow::{check_work_proof, gen_extrinsic_proof, calc_pow_target};
use yee_sharding::{ShardingDigestItem, ScaleOutPhaseDigestItem};
use crate::verifier::check_scale;
use primitives::H256;
use ansi_term::Colour;
use yee_context::Context;

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
	/// extrinsic proof
	pub xts_proof: Vec<u8>,
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

pub struct DefaultJobManager<B, C, E, AccountId, AuthorityId, I> where
	B: Block,
{
	client: Arc<C>,
	env: Arc<E>,
	inherent_data_providers: InherentDataProviders,
	authority_id: AuthorityId,
	block_import: Arc<I>,
	shard_extra: ShardExtra<AccountId>,
	context: Context<B>,
	phantom: PhantomData<B>,
}

impl<B, C, E, AccountId, AuthorityId, I> DefaultJobManager<B, C, E, AccountId, AuthorityId, I> where
	B: Block,
	C: ChainHead<B>,
	E: Environment<B> + 'static,
	<E as Environment<B>>::Proposer: Proposer<B>,
	<E as Environment<B>>::Error: Debug,
	<<E as Environment<B>>::Proposer as Proposer<B>>::Create: IntoFuture<Item=(B, Vec<bool>)>,
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
		shard_extra: ShardExtra<AccountId>,
		context: Context<B>,
	) -> Self {
		Self {
			client,
			env,
			inherent_data_providers,
			authority_id,
			block_import,
			shard_extra,
			context,
			phantom: PhantomData,
		}
	}
}

impl<B, C, E, AccountId, AuthorityId, I> JobManager for DefaultJobManager<B, C, E, AccountId, AuthorityId, I>
	where B: Block,
	      DigestItemFor<B>: super::CompatibleDigestItem<B, AuthorityId> + ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<NumberFor<B>, u16>,
	      C: ChainHead<B> + Send + Sync + 'static,
	      C: HeaderBackend<B> + ProvideRuntimeApi,
	      <C as ProvideRuntimeApi>::Api: YeePOWApi<B>,
	      E: Environment<B> + 'static + Send + Sync,
	      <E as Environment<B>>::Proposer: Proposer<B>,
	      <E as Environment<B>>::Error: Debug,
	      <<E as Environment<B>>::Proposer as Proposer<B>>::Create: IntoFuture<Item=(B, Vec<bool>)>,
	      <<<E as Environment<B>>::Proposer as Proposer<B>>::Create as IntoFuture>::Future: Send + 'static,
	      AuthorityId: Decode + Encode + Clone + Send + Sync + 'static,
	      AccountId: Decode + Encode + Clone + Send + Sync + 'static,
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
		let context = self.context.clone();

		let build_job = move |(block, exe_result): (B, Vec<bool>)| {
			let (header, body) = block.deconstruct();
			let header_num = header.number().clone();
			let header_pre_hash = header.hash();
			let timestamp = timestamp_now()?;
			let pow_target = calc_pow_target(client, &header, timestamp, &context)?;
			let authority_id = authority_id;
			let work_proof = WorkProof::Unknown;
			// generate proof
			let (relay_proof, proof) = gen_extrinsic_proof::<B>(&header, &body, exe_result);

			let pow_seal = PowSeal {
				authority_id,
				pow_target,
				timestamp,
				work_proof,
				relay_proof,
			};
			let mut header_with_pow_seal = header.clone();
			let item = <DigestItemFor<B> as CompatibleDigestItem<B, AuthorityId>>::pow_seal(pow_seal.clone());
			header_with_pow_seal.digest_mut().push(item);

			let hash = header_with_pow_seal.hash();

			info!("job {} @ {:?}, pow target: {:#x}", header_num, header_pre_hash, pow_target);

			Ok(DefaultJob {
				hash,
				header,
				body,
				digest_item: pow_seal,
				xts_proof: proof,
			})
		};

		Box::new(proposer.propose(inherent_data, Duration::from_secs(10)).into_future()
			.map_err(to_common_error).and_then(build_job))
	}

	fn submit_job(&self, job: Self::Job) -> Box<dyn Future<Item=<Self::Job as Job>::Hash, Error=consensus_common::Error> + Send>{

		let block_import = self.block_import.clone();

		let check_job = move |job: Self::Job| -> Result<<Self::Job as Job>::Hash, consensus_common::Error>{
			let number = &job.header.number().clone();
			let (post_digest, hash) = check_work_proof(&job.header, &job.digest_item)?;

			check_scale::<B, AccountId>(&job.header, self.shard_extra.clone())?;

			let import_block: ImportBlock<B> = ImportBlock {
				origin: BlockOrigin::Own,
				header: job.header,
				justification: None,
				proof: Some(job.xts_proof),
				post_digests: vec![post_digest],
				body: Some(job.body),
				finalized: false,
				auxiliary: Vec::new(),
				fork_choice: ForkChoiceStrategy::LongestChain,
			};
			block_import.import_block(import_block, Default::default())?;
			info!("{} @ {} {:?}", Colour::Green.bold().paint("Block mined"), number, hash);
			Ok(hash)
		};

		Box::new(check_job(job).into_future())

	}
}

fn timestamp_now() -> Result<u64, consensus_common::Error> {
	Ok(SystemTime::now().duration_since(UNIX_EPOCH)
		.map_err(to_common_error)?.as_millis() as u64)
}
