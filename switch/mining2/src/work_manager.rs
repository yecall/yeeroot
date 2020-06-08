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
use serde::export::fmt::Debug;
use yee_serde_hex::SerdeHex;
use tokio::runtime::Runtime;
use std::thread;
use parity_codec::{Decode, Encode};
use yee_sharding::{GENERATED_MODULE_LOG_PREFIX, GENERATED_SHARDING_PREFIX};
use yee_consensus_pow_primitives::PowTarget;
use yee_consensus_pow::{MiningHash, MiningAlgorithm, OriginalMerkleProof, CompactMerkleProof};
use merkle_light::merkle::MerkleTree;
use runtime_primitives::traits::Hash as HashT;
use std::iter::FromIterator;
use std::ops::Add;
use crate::error;
use log::{info, debug};
use futures::future;
use jsonrpc_core::BoxFuture;

const EXTRA_DATA: &str = "yee-switch";
const RAW_WORK_LIFE: Duration = Duration::from_secs(60);
const REFRESH_JOB_DELAY: Duration = Duration::from_secs(2);

pub trait WorkManager {
	type Hashing: HashT;

	// notice that work may not refresh
	fn get_work(&self) -> error::Result<Work<<Self::Hashing as HashT>::Output>>;

	fn get_work_by_merkle(&self, root: <Self::Hashing as HashT>::Output) -> error::Result<Work<<Self::Hashing as HashT>::Output>>;

	fn submit_work(&self, work: Work<<Self::Hashing as HashT>::Output>) -> error::Result<()>;

	fn submit_work_future(&self, work: Work<<Self::Hashing as HashT>::Output>) -> Box<dyn Future<Item=(), Error=error::Error> + Send>;
}

impl<Number, AuthorityId, Hashing> WorkManager for DefaultWorkManager<Number, AuthorityId, Hashing> where
	Number: Clone + Debug + SerdeHex + DeserializeOwned + Send + Sync + 'static,
	AuthorityId: Clone + Debug + DeserializeOwned + Send + Sync + 'static,
	Hashing: HashT + Send + Sync + 'static,
	Hashing::Output: Ord + Encode + Decode,
{
	type Hashing = Hashing;

	fn get_work(&self) -> error::Result<Work<<Self::Hashing as HashT>::Output>> {
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

		info!("Raw work: shard_count: {}, shard_jobs: {:?}",
			  raw_work.shard_count,
			  raw_work.shard_jobs.iter()
				  .map(|(actual_shard_num, (config_shard_num, job))| (*actual_shard_num, (*config_shard_num, job.hash.clone())))
				  .collect::<HashMap<u16, (u16, Hashing::Output)>>()
		);

		Ok(raw_work.work.expect("qed"))
	}

	fn get_work_by_merkle(&self, root: <Self::Hashing as HashT>::Output) -> error::Result<Work<<Self::Hashing as HashT>::Output>> {
		let raw_work = Self::get_cache(self.work_cache.clone(), &root).ok_or(error::Error::from(error::ErrorKind::WorkExpired))?;
		match raw_work.work {
			Some(work) => Ok(work),
			None => Err(error::ErrorKind::WorkExpired.into())
		}
	}

	fn submit_work(&self, work: Work<<Self::Hashing as HashT>::Output>) -> error::Result<()> {
		self.accept_work(work)
	}

	fn submit_work_future(&self, work: Work<<Self::Hashing as HashT>::Output>) -> Box<dyn Future<Item=(), Error=error::Error> + Send> {
		self.accept_work_future(work)
	}
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct Work<Hash> {
	pub merkle_root: Hash,
	pub extra_data: Vec<u8>,
	pub target: PowTarget,
	pub nonce: Option<u64>,
	pub nonce_target: Option<PowTarget>,
}

#[derive(Debug, Clone)]
pub struct RawWork<Number: SerdeHex, AuthorityId, Hashing> where
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
	work: Option<Work<Hashing::Output>>,

}

impl<Number: SerdeHex, AuthorityId, Hashing> RawWork<Number, AuthorityId, Hashing> where
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

		let merkle_tree: MerkleTree<MiningHash<Hashing>, MiningAlgorithm<Hashing>> =
			MerkleTree::from_iter(item_list);

		let work = Work {
			merkle_root: merkle_tree.root(),
			extra_data: EXTRA_DATA.as_bytes().to_vec(),
			target: max_target,
			nonce: None,
			nonce_target: None,
		};

		self.merkle_tree = Some(merkle_tree);
		self.work = Some(work);
	}
}

pub struct DefaultWorkManager<Number: SerdeHex, AuthorityId, Hashing> where
	Hashing: HashT,
	Hashing::Output: Ord + Encode + Decode,
{
	config: Config,
	jobs: Arc<RwLock<HashMap<u16, Job<Hashing::Output, Number, AuthorityId>>>>,
	work_cache: Arc<RwLock<HashMap<Hashing::Output, (RawWork<Number, AuthorityId, Hashing>, Instant)>>>,
}

impl<Number, AuthorityId, Hashing> DefaultWorkManager<Number, AuthorityId, Hashing>
	where
		Number: Send + Sync + Debug + DeserializeOwned + SerdeHex + Clone + 'static,
		AuthorityId: Send + Sync + Debug + DeserializeOwned + Clone + 'static,
		Hashing: HashT + Send + Sync + 'static,
		Hashing::Output: Ord + DeserializeOwned + Encode + Decode + Send + Sync + 'static,
{
	pub fn new(config: Config) -> Self {
		Self {
			config,
			jobs: Arc::new(RwLock::new(HashMap::new())),
			work_cache: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	pub fn start(&mut self) -> error::Result<()> {
		let mut runtime = Runtime::new().map_err(|e| format!("{:?}", e))?;

		let executor = runtime.executor();

		let shards = self.config.shards.clone();

		let jobs = self.jobs.clone();

		let _thread = thread::Builder::new().name("job manager".to_string()).spawn(move || {

			//fetch job from all the shards
			for (shard_num, shard) in &shards {
				let config_shard_num: u16 = shard_num.parse().expect("qed");

				let shard = shard.clone();
				let jobs = jobs.clone();

				let task = Interval::new(Instant::now(), Duration::from_secs(1)).for_each(move |_instant| {
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

	fn accept_work(&self, work: Work<Hashing::Output>) -> error::Result<()> {
		let nonce = work.nonce.expect("qed");
		let nonce_target = work.nonce_target.expect("qed");
		let target = work.target.clone();

		info!("accept_work: merkle_root: {:?}, nonce: {}, nonce_target: {:#x}, target: {:#x}", work.merkle_root, nonce, nonce_target, target);

		let merkle_root = &work.merkle_root;

		let raw_work = Self::get_cache(self.work_cache.clone(), merkle_root).ok_or(error::Error::from(error::ErrorKind::WorkExpired))?;

		let shard_count = raw_work.shard_count;

		let shard_jobs = raw_work.shard_jobs;
		let merkle_tree = raw_work.merkle_tree.expect("qed");

		let merkle_tree = Arc::new(merkle_tree);
		let work = Arc::new(work);

		let mut tasks = Vec::new();

		for actual_shard_num in 0..shard_count {
			if let Some((config_shard_num, job)) = shard_jobs.get(&actual_shard_num) {
				let job_target = job.digest_item.pow_target;

				let config_shard_num = config_shard_num.clone();
				let job = job.clone();
				let merkle_tree = merkle_tree.clone();
				let work = work.clone();

				let shard = Arc::new(self.config.shards.get(&format!("{}", config_shard_num)).expect("qed").to_owned());
				let shard2 = shard.clone();

				let jobs = self.jobs.clone();

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
						future::ok(job_result)
					} else {
						warn!("nonce_target: {:#x}, job_target: {:#x}", nonce_target, job_target);
						future::err(error::Error::from(error::ErrorKind::TargetNotAccpect))
					}
				}).and_then(move |job_result| {
					info!("New block mined: actual_shard_num: {}, config_shard_num: {}, nonce_target: {:#x}, job_target: {:#x}", actual_shard_num, config_shard_num, nonce_target, job_target);
					Self::submit_job_future(&shard, job_result)
				}).and_then(move |result| {
					debug!("Job submitted: actual_shard_num: {}, config_shard_num: {}, new_block_hash: {:?}", actual_shard_num, config_shard_num, result);
					Delay::new(Instant::now().add(REFRESH_JOB_DELAY)).then(move |_| {
						Self::get_job_future(&shard2).then(move |job| {
							info!("Job refreshed: actual_shard_num: {}, config_shard_num: {}, job_hash: {:?}", actual_shard_num, config_shard_num, job.as_ref().map(|job| job.hash));
							match job {
								Ok(job) => jobs.write().insert(config_shard_num, job),
								Err(_e) => jobs.write().remove(&config_shard_num),
							};
							Ok(())
						})
					})
				});
				tasks.push(task);
			}
		}

		let task = future::join_all(tasks).map(|_| ()).map_err(|e| warn!("{:?}", e));

		tokio::run(task);

		Ok(())
	}

	fn accept_work_future(&self, work: Work<Hashing::Output>) -> Box<dyn Future<Item=(), Error=error::Error> + Send> {
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

		for actual_shard_num in 0..shard_count {
			if let Some((config_shard_num, job)) = shard_jobs.get(&actual_shard_num) {
				let job_target = job.digest_item.pow_target;

				let config_shard_num = config_shard_num.clone();
				let job = job.clone();
				let merkle_tree = merkle_tree.clone();
				let work = work.clone();

				let shard = Arc::new(self.config.shards.get(&format!("{}", config_shard_num)).expect("qed").to_owned());
				let shard2 = shard.clone();

				let jobs = self.jobs.clone();

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
						future::ok(job_result)
					} else {
						warn!("nonce_target: {:#x}, job_target: {:#x}", nonce_target, job_target);
						future::err(error::Error::from(error::ErrorKind::TargetNotAccpect))
					}
				}).and_then(move |job_result| {
					info!("New block mined: actual_shard_num: {}, config_shard_num: {}, nonce_target: {:#x}, job_target: {:#x}", actual_shard_num, config_shard_num, nonce_target, job_target);
					Self::submit_job_future(&shard, job_result)
				}).and_then(move |result| {
					info!("Job submitted: actual_shard_num: {}, config_shard_num: {}, new_block_hash: {:?}", actual_shard_num, config_shard_num, result);
					Delay::new(Instant::now().add(REFRESH_JOB_DELAY)).then(move |_| {
						Self::get_job_future(&shard2).then(move |job| {
							info!("Job refreshed: actual_shard_num: {}, config_shard_num: {}, job_hash: {:?}", actual_shard_num, config_shard_num, job.as_ref().map(|job| job.hash));
							match job {
								Ok(job) => jobs.write().insert(config_shard_num, job),
								Err(_e) => jobs.write().remove(&config_shard_num),
							};
							Ok(())
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

	fn put_cache(cache: Arc<RwLock<HashMap<Hashing::Output, (RawWork<Number, AuthorityId, Hashing>, Instant)>>>, raw_work: RawWork<Number, AuthorityId, Hashing>) {
		let now = Instant::now();

		let expire_at = now.add(RAW_WORK_LIFE);

		let mut cache = cache.write();

		cache.insert(raw_work.work.as_ref().expect("qed").merkle_root.clone(), (raw_work.clone(), expire_at));

		let expired: Vec<Hashing::Output> = cache.iter()
			.filter(|(_k, v)| (**v).1.lt(&now))
			.map(|(k, _v)| k).cloned().collect();

		for i in expired {
			cache.remove(&i);
		}
	}

	fn get_cache(cache: Arc<RwLock<HashMap<Hashing::Output, (RawWork<Number, AuthorityId, Hashing>, Instant)>>>, hash: &Hashing::Output) -> Option<RawWork<Number, AuthorityId, Hashing>> {
		let now = Instant::now();
		cache.read().get(&hash).and_then(|x| {
			if x.1.lt(&now) {
				None
			} else {
				Some(x.0.to_owned())
			}
		})
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