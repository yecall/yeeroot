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

//! Work manager
//! fetch job from rpc of all the shards, maintain the jobs and provide get_work

use yee_primitives::{Config, Shard};
use jsonrpc_core_client::TypedClient;
use rand::Rng;
use futures::Future;
use futures::Stream;
use yee_rpc::mining::primitives::{Job, JobResult, ResultDigestItem, WorkProof, ProofMulti};
use serde::de::DeserializeOwned;
use serde::{Serialize, Deserialize};
use tokio::timer::Interval;
use tokio::timer::Delay;
use std::time::{Instant, Duration};
use log::warn;
use parking_lot::RwLock;
use std::sync::Arc;
use std::collections::HashMap;
use serde::export::fmt::{Debug, Display};
use yee_serde_hex::SerdeHex;
use tokio::runtime::Runtime;
use std::thread;
use parity_codec::{Decode, Encode};
use yee_sharding::{GENERATED_MODULE_LOG_PREFIX, GENERATED_SHARDING_PREFIX};
use yee_consensus_pow_primitives::PowTarget;
use yee_consensus_pow::{MiningHash, MiningAlgorithm, ExtraData, OriginalMerkleProof, CompactMerkleProof};
use merkle_light::merkle::MerkleTree;
use runtime_primitives::traits::{BlakeTwo256, Hash as HashT};
use std::iter::FromIterator;
use std::ops::Add;
use crate::error;
use log::{info, debug};
use futures::future;
use jsonrpc_core::BoxFuture;
use yee_runtime::BlockNumber;
use futures::future::{Loop, Either};
use rand::thread_rng;
use rand::seq::SliceRandom;
use lru::LruCache;
use chashmap::CHashMap;

const REFRESH_JOB_DELAY: Duration = Duration::from_millis(1000);
const REFRESH_MAX_TRY_TIMES: usize = 3;
const JOB_LOCK_LIFE: Duration = Duration::from_millis(3000);

pub trait WorkManager {
	type Hashing: HashT;
	type Number: Debug + Clone;

	// notice that work may not refresh
	fn get_work(&self) -> error::Result<Work<<Self::Hashing as HashT>::Output, Self::Number>>;

	fn get_work_by_merkle(&self, root: <Self::Hashing as HashT>::Output) -> error::Result<Work<<Self::Hashing as HashT>::Output, Self::Number>>;

	fn submit_work(&self, work: Work<<Self::Hashing as HashT>::Output, Self::Number>) -> error::Result<()>;

	fn submit_work_future(&self, work: Work<<Self::Hashing as HashT>::Output, Self::Number>) -> Box<dyn Future<Item=(), Error=error::Error> + Send>;
}

impl<Number, AuthorityId, Hashing> WorkManager for DefaultWorkManager<Number, AuthorityId, Hashing> where
	Number: Clone + Debug + SerdeHex + DeserializeOwned + Send + Sync + PartialOrd + Display + 'static,
	AuthorityId: Clone + Debug + DeserializeOwned + Send + Sync + 'static,
	Hashing: HashT + Send + Sync + 'static,
	Hashing::Output: Ord + Encode + Decode,
{
	type Hashing = Hashing;
	type Number = Number;

	fn get_work(&self) -> error::Result<Work<<Self::Hashing as HashT>::Output, Self::Number>> {
		let jobs = &*self.jobs.read();

		debug!("jobs: {:?}", jobs);

		if jobs.len() == 0 {
			return Err(error::ErrorKind::ShardsDown.into());
		}

		// get shard info of each shard
		let shard_info_map: HashMap<u16, (u16, u16)> = jobs.iter().filter_map(|(config_shard_num, job)| {
			parse_shard_info(job).map(|shard_info| (*config_shard_num, shard_info))
		}).collect();

		// calc shard count
		let shard_count = shard_info_map.iter().map(|(_, (_, shard_count))| *shard_count).max();

		let shard_count = match shard_count {
			Some(shard_count) => shard_count,
			None => return Err(error::ErrorKind::ShardsDown.into()),
		};

		// calc shard jobs
		let mut shard_jobs: HashMap<u16, Vec<(u16, Job<Hashing::Output, Number, AuthorityId>)>> = HashMap::new();

		for (&config_shard_num, job) in jobs {
			if let Some((actual_shard_num, actual_shard_count)) = shard_info_map.get(&config_shard_num) {
				//normal or scale out
				if config_shard_num == *actual_shard_num || config_shard_num == *actual_shard_num + *actual_shard_count {
					let entry = shard_jobs.entry(*actual_shard_num).or_insert(vec![]);
					entry.push((config_shard_num, (*job).clone()));
				}
			}
		}

		debug!("shard_jobs: {:?}", shard_jobs);

		//random job if needed
		let mut rng = rand::thread_rng();
		let shard_jobs: HashMap<u16, (u16, Job<Hashing::Output, Number, AuthorityId>)> = shard_jobs.iter().map(|(actual_shard_num, items)| {
			let i = if items.len() == 1 {
				0
			} else {
				rng.gen_range(0, items.len())
			};
			let item = items[i].clone();
			(*actual_shard_num, item)
		}).collect();

		// update work
		let mut raw_work = RawWork::<_, _, Hashing> {
			shard_count,
			shard_jobs,
			merkle_tree: None,
			work: None,
		};
		raw_work.compile();

		Self::put_cache(self.work_cache.clone(), raw_work.clone());

		debug!("Raw work: shard_count: {}, shard_jobs: {:?}",
			  raw_work.shard_count,
			  raw_work.shard_jobs.iter()
				  .map(|(actual_shard_num, (config_shard_num, job))| (*actual_shard_num, (*config_shard_num, job.hash.clone())))
				  .collect::<HashMap<u16, (u16, Hashing::Output)>>()
		);

		Ok(raw_work.work.expect("qed"))
	}

	fn get_work_by_merkle(&self, root: <Self::Hashing as HashT>::Output) -> error::Result<Work<<Self::Hashing as HashT>::Output, Self::Number>> {
		let raw_work = Self::get_cache(self.work_cache.clone(), &root).ok_or(error::Error::from(error::ErrorKind::WorkExpired))?;
		match raw_work.work {
			Some(work) => Ok(work),
			None => Err(error::ErrorKind::WorkExpired.into())
		}
	}

	fn submit_work(&self, work: Work<<Self::Hashing as HashT>::Output, Self::Number>) -> error::Result<()> {
		tokio::run(self.accept_work_future(work).map_err(|_|()));
		Ok(())
	}

	fn submit_work_future(&self, work: Work<<Self::Hashing as HashT>::Output, Self::Number>) -> Box<dyn Future<Item=(), Error=error::Error> + Send> {
		self.accept_work_future(work)
	}
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct Work<Hash, Number> {
	pub merkle_root: Hash,
	pub extra_data: ExtraData,
	pub target: PowTarget,
	pub shard_count: u16,
	pub shard_block_number: HashMap<u16, Number>,
	pub nonce: Option<u64>,
	pub nonce_target: Option<PowTarget>,
}

#[derive(Debug, Clone)]
pub struct RawWork<Number, AuthorityId, Hashing> where
	Number: SerdeHex,
	Hashing: HashT,
	Hashing::Output: Ord + Encode + Decode,
{
	// will detect shard count according to header data of jobs
	shard_count: u16,

	// actual_shard_num -> (config_shard_num, job)
	// e.g. when scaling from 4 to 8, config_shard_num is 4, while actual_shard_num = 0
	shard_jobs: HashMap<u16, (u16, Job<Hashing::Output, Number, AuthorityId>)>,

	// merkle tree
	merkle_tree: Option<MerkleTree<MiningHash<Hashing>, MiningAlgorithm<Hashing>>>,

	// pow work
	work: Option<Work<Hashing::Output, Number>>,

}

impl<Number, AuthorityId, Hashing> RawWork<Number, AuthorityId, Hashing> where
	Number: SerdeHex + Clone,
	Hashing: HashT,
	Hashing::Output: Ord + Encode + Decode,
{
	//update merkle_tree and work
	fn compile(&mut self) {

		// calc work
		let item_and_target_list: Vec<(Hashing::Output, PowTarget)> = (0..self.shard_count).map(|x| {
			let job = self.shard_jobs.get(&x);
			let item_and_target = match job {
				Some((_, job)) => (job.hash, job.digest_item.pow_target),
				None => (Default::default(), Default::default()),
			};
			item_and_target
		}).collect();

		let item_list: Vec<Hashing::Output> = item_and_target_list.iter().map(|(item, _)| item.clone()).collect();

		let max_target = item_and_target_list.iter().map(|(_, target)| target).max().expect("qed").clone();

		let shard_count = self.shard_count;

		let shard_block_number = self.shard_jobs.iter().map(|(k,(_, v))| {
			(*k, v.header.number.clone())
		}).collect::<HashMap<_, _>>();

		let merkle_tree: MerkleTree<MiningHash<Hashing>, MiningAlgorithm<Hashing>> =
			MerkleTree::from_iter(item_list);
		let extra = [0u8; 36];
		let extra = &extra[..];
		let h = BlakeTwo256::hash(extra);
		let mut e_d = [0u8; 40];
		for i in 0..4 {
			e_d[i + 36] = h[i];
		}
		let work = Work {
			merkle_root: merkle_tree.root(),
			extra_data: ExtraData::from(e_d),
			target: max_target,
			shard_count,
			shard_block_number,
			nonce: None,
			nonce_target: None,
		};

		self.merkle_tree = Some(merkle_tree);
		self.work = Some(work);
	}
}

pub struct WorkManagerConfig{
	pub job_refresh_interval: u64,
	pub job_cache_size: u32,
}

pub struct DefaultWorkManager<Number, AuthorityId, Hashing> where
	Number: SerdeHex + Clone,
	Hashing: HashT,
	Hashing::Output: Ord + Encode + Decode,
{
	config: Config,
	work_manager_config: WorkManagerConfig,
	jobs: Arc<RwLock<HashMap<u16, Job<Hashing::Output, Number, AuthorityId>>>>,
	work_cache: Arc<RwLock<LruCache<Hashing::Output, RawWork<Number, AuthorityId, Hashing>>>>,
	submitted_number: Arc<CHashMap<u16, (Number, Instant)>>,
}

impl<Number, AuthorityId, Hashing> DefaultWorkManager<Number, AuthorityId, Hashing>
	where
		Number: Send + Sync + Debug + DeserializeOwned + SerdeHex + Clone + PartialOrd + Display + 'static,
		AuthorityId: Send + Sync + Debug + DeserializeOwned + Clone + 'static,
		Hashing: HashT + Send + Sync + 'static,
		Hashing::Output: Ord + DeserializeOwned + Encode + Decode + Send + Sync + 'static,
{
	pub fn new(config: Config, work_manager_config: WorkManagerConfig) -> Self {

		let job_cache_size = work_manager_config.job_cache_size as usize;
		Self {
			config,
			work_manager_config,
			jobs: Arc::new(RwLock::new(HashMap::new())),
			work_cache: Arc::new(RwLock::new(LruCache::new(job_cache_size))),
			submitted_number: Arc::new(CHashMap::new()),
		}
	}

	pub fn start(&mut self) -> error::Result<()> {
		let mut runtime = Runtime::new().map_err(|e| format!("{:?}", e))?;

		let executor = runtime.executor();

		let shards = self.config.shards.clone();

		let jobs = self.jobs.clone();

		let job_refresh_interval = self.work_manager_config.job_refresh_interval;

		let _thread = thread::Builder::new().name("job manager".to_string()).spawn(move || {

			//fetch job from all the shards
			for (shard_num, shard) in &shards {
				let config_shard_num: u16 = shard_num.parse().expect("qed");

				let shard = shard.clone();
				let jobs = jobs.clone();

				let task = Interval::new(Instant::now(), Duration::from_millis(job_refresh_interval)).for_each(move |_instant| {
					let jobs = jobs.clone();
					Self::get_job_future(&shard).then(move |job| {
						match job {
							Ok(job) => jobs.write().insert(config_shard_num, job),
							Err(_e) => jobs.write().remove(&config_shard_num),
						};
						Ok(())
					})
				}).map_err(|e| warn!("{:?}", e));

				executor.spawn(task);
			}

			let (_signal, exit) = exit_future::signal();
			runtime.block_on(exit);
		});

		Ok(())
	}

	fn accept_work_future(&self, work: Work<Hashing::Output, Number>) -> Box<dyn Future<Item=(), Error=error::Error> + Send> {
		let nonce = work.nonce.expect("qed");
		let nonce_target = work.nonce_target.expect("qed");

		info!("Accept work: merkle_root: {:?}, nonce: {}, nonce_target: {:#x}", work.merkle_root, nonce, nonce_target);

		let merkle_root = &work.merkle_root;

		let raw_work = match Self::get_cache(self.work_cache.clone(), merkle_root) {
			Some(raw_work) => raw_work,
			None => return Box::new(future::err(error::ErrorKind::WorkExpired.into())),
		};

		let shard_count = raw_work.shard_count;

		let shard_jobs = raw_work.shard_jobs;
		let merkle_tree = raw_work.merkle_tree.expect("qed");

		let merkle_tree = Arc::new(merkle_tree);
		let work = Arc::new(work);

		let mut tasks = Vec::new();

		let mut shard_num_list = (0..shard_count).collect::<Vec<_>>();
		let mut rng = thread_rng();
		shard_num_list.shuffle(&mut rng);

		for actual_shard_num in shard_num_list {
			if let Some((config_shard_num, job)) = shard_jobs.get(&actual_shard_num) {
				let job_target = job.digest_item.pow_target;

				let config_shard_num = config_shard_num.clone();
				let job = job.clone();
				let merkle_tree = merkle_tree.clone();
				let work = work.clone();

				let shard = Arc::new(self.config.shards.get(&format!("{}", config_shard_num)).expect("qed").to_owned());
				let shard2 = shard.clone();

				let jobs = self.jobs.clone();
				let submitted_number = self.submitted_number.clone();
				let submitted_number2 = self.submitted_number.clone();

				let task = future::lazy(move || {
					if nonce_target <= job_target {
						let merkle_proof = merkle_tree.gen_proof(actual_shard_num as usize);

						let merkle_proof = OriginalMerkleProof {
							proof: merkle_proof,
							num: actual_shard_num,
							count: shard_count,
						};
						let merkle_proof: CompactMerkleProof<Hashing> = merkle_proof.into();

						let job_result = JobResult {
							hash: job.hash,
							digest_item: ResultDigestItem {
								work_proof: WorkProof::Multi(ProofMulti {
									extra_data: work.extra_data.clone(),
									merkle_root: work.merkle_root.clone(),
									merkle_proof: merkle_proof.proof,
									nonce,
								})
							},
						};
						future::ok((job_result, job))
					} else {
						warn!("nonce_target: {:#x}, job_target: {:#x}", nonce_target, job_target);
						future::err(error::Error::from(error::ErrorKind::TargetNotAccept))
					}
				}).and_then(move |(job_result, job)| {
					info!("New block mined: actual_shard_num: {}, config_shard_num: {}, number: {}, nonce_target: {:#x}, job_target: {:#x}",
						  actual_shard_num, config_shard_num, job.header.number, nonce_target, job_target);

					// check number

					let submit_repeated = match submitted_number.get(&config_shard_num).as_ref().map(|x|&**x){
						Some((number, instant)) => &job.header.number == number && instant.elapsed() < JOB_LOCK_LIFE,
						_ => false,
					};

					if submit_repeated {
						warn!("Job repeated: actual_shard_num: {}, config_shard_num: {}, number: {}", actual_shard_num, config_shard_num, job.header.number);
						Box::new(future::err(error::Error::from(error::ErrorKind::SubmitRepeated))) as Box<dyn Future<Item=(Hashing::Output, Job<Hashing::Output, Number, AuthorityId>), Error=error::Error> + Send>
					}else{
						Box::new(Self::submit_job_future(&shard, job_result).map(|result| (result, job)))
					}

				}).and_then(move |(result, job)| {
					info!("Job submitted: actual_shard_num: {}, config_shard_num: {}, number: {}, new_block_hash: {:?}",
						   actual_shard_num, config_shard_num, job.header.number, result);

					// save number
					let should_save_number = match submitted_number2.get(&config_shard_num).as_ref().map(|x|&**x){
						Some((number, instant)) => &job.header.number != number,
						_ => true,
					};
					if should_save_number {
						submitted_number2.insert(config_shard_num, (job.header.number.clone(), Instant::now()));
					}

					future::loop_fn(0usize, move |i| {
						let job_block_number = job.header.number.clone();
						let jobs = jobs.clone();
						Self::get_job_future(&shard2).then(move |new_job| {
							let delay_continue = Delay::new(Instant::now().add(REFRESH_JOB_DELAY)).then(move |_|{
								future::ok(Loop::Continue(i+1))
							});
							match new_job {
								Ok(new_job) => {
									if new_job.header.number > job_block_number {
										info!("Job refreshed: actual_shard_num: {}, config_shard_num: {}, number: {}, job_hash: {:?}",
											  actual_shard_num, config_shard_num, new_job.header.number, new_job.hash);
										jobs.write().insert(config_shard_num, new_job);
										Either::A(future::ok(Loop::Break(())))
									}else{
										if i < REFRESH_MAX_TRY_TIMES {
											info!("Job not refreshed: number not incr, delay to continue: {}", i);
											Either::B(delay_continue)
										} else {
											info!("Job not refreshed: number not incr, break");
											Either::A(future::ok(Loop::Break(())))
										}
									}
								},
								Err(e) => {
									if i < REFRESH_MAX_TRY_TIMES {
										info!("Job not refreshed: encounter error, delay to continue: {}", i);
										Either::B(delay_continue)
									} else {
										info!("Job not refreshed: encounter error, break");
										Either::A(future::ok(Loop::Break(())))
									}
								}
							}
						})
					})
				});
				tasks.push(task);
			}
		}

		let tasks = future::join_all(tasks)
			.map(|_| ())
			.map_err(|e| {
				warn!("{:?}", e);
				e
			});
		Box::new(tasks)
	}

	fn submit_job(&self, config_shard_num: u16, job_result: JobResult<Hashing::Output>) -> error::Result<Hashing::Output> {
		let shard = self.config.shards.get(&format!("{}", config_shard_num)).ok_or(error::Error::from(error::ErrorKind::ShardNotFound))?;
		let rpc = &shard.rpc;
		let mut rng = rand::thread_rng();
		let i = rng.gen_range(0, rpc.len());
		let uri = rpc.get(i).expect("qed");
		let method = "mining_submitJob";

		let (tx, rx) = std::sync::mpsc::channel::<Hashing::Output>();

		let task = jsonrpc_core_client::transports::http::connect(uri)
			.and_then(move |client: TypedClient| {
				client.call_method(&method, "returns", (job_result, )).and_then(move |result| {
					tx.send(result);
					Ok(())
				})
			}).map_err(move |e| warn!("Submit job error: config_shard_num:{} {:?}", config_shard_num, e));

		tokio::run(task);

		rx.recv_timeout(Duration::from_secs(3)).map_err(|e| format!("Submit job error: {:?}", e).into())
	}

	fn get_job(&self, config_shard_num: u16) -> error::Result<Job<Hashing::Output, Number, AuthorityId>> {
		let shard = self.config.shards.get(&format!("{}", config_shard_num)).ok_or(error::Error::from(error::ErrorKind::ShardNotFound))?;
		let rpc = &shard.rpc;
		let mut rng = rand::thread_rng();
		let i = rng.gen_range(0, rpc.len());
		let uri = rpc.get(i).expect("qed");
		let method = "mining_getJob";

		let (tx, rx) = std::sync::mpsc::channel::<Job<Hashing::Output, Number, AuthorityId>>();

		let task = jsonrpc_core_client::transports::http::connect(uri)
			.and_then(move |client: TypedClient| {
				client.call_method(&method, "returns", ()).and_then(move |result| {
					tx.send(result);
					Ok(())
				})
			}).map_err(move |e| warn!("Get job error: config_shard_num: {} {:?}", config_shard_num, e));

		tokio::run(task);

		rx.recv_timeout(Duration::from_secs(3)).map_err(|e| format!("Get job error: {:?}", e).into())
	}

	fn submit_job_future(shard: &Shard, job_result: JobResult<Hashing::Output>) -> Box<dyn Future<Item=Hashing::Output, Error=error::Error> + Send> {
		let rpc = &shard.rpc;
		let mut rng = rand::thread_rng();
		let i = rng.gen_range(0, rpc.len());
		let uri = rpc.get(i).expect("qed");
		let method = "mining_submitJob";

		Box::new(jsonrpc_core_client::transports::http::connect(uri)
			.and_then(move |client: TypedClient| {
				client.call_method(&method, "returns", (job_result, )).and_then(move |result| {
					Ok(result)
				})
			}).map_err(|e| format!("Submit job error: {:?}", e).into()))
	}

	fn get_job_future(shard: &Shard) -> Box<dyn Future<Item=Job<Hashing::Output, Number, AuthorityId>, Error=error::Error> + Send> {
		let rpc = &shard.rpc;
		let mut rng = rand::thread_rng();
		let i = rng.gen_range(0, rpc.len());
		let uri = rpc.get(i).expect("qed");
		let method = "mining_getJob";

		Box::new(jsonrpc_core_client::transports::http::connect(uri)
			.and_then(move |client: TypedClient| {
				client.call_method(&method, "returns", ()).and_then(move |result| {
					Ok(result)
				})
			}).map_err(|e| format!("Get job error: {:?}", e).into()))
	}

	fn put_cache(cache: Arc<RwLock<LruCache<Hashing::Output, RawWork<Number, AuthorityId, Hashing>>>>, raw_work: RawWork<Number, AuthorityId, Hashing>) {

		let merkle_root = raw_work.work.as_ref().expect("qed").merkle_root.clone();

		cache.write().put(merkle_root, raw_work);
	}

	fn get_cache(cache: Arc<RwLock<LruCache<Hashing::Output, RawWork<Number, AuthorityId, Hashing>>>>, hash: &Hashing::Output) -> Option<RawWork<Number, AuthorityId, Hashing>> {

		cache.write().get(hash).cloned()
	}
}

fn parse_shard_info<Hash, Number, AuthorityId>(job: &Job<Hash, Number, AuthorityId>) -> Option<(u16, u16)> where
	Number: SerdeHex
{
	job.header.digest.logs.iter().filter_map(|x| {
		let item = &x.0;
		if item.len() > 0 && item[0] == 0 {
			let input = &mut &item[1..];
			let data: Vec<u8> = Decode::decode(input)?;
			if data.len() >= 6 && data[0] == GENERATED_MODULE_LOG_PREFIX && data[1] == GENERATED_SHARDING_PREFIX {
				let input = &mut &data[2..];
				let shard_num: u16 = Decode::decode(input)?;
				let shard_count: u16 = Decode::decode(input)?;
				Some((shard_num, shard_count))
			} else {
				None
			}
		} else {
			None
		}
	}).next()
}