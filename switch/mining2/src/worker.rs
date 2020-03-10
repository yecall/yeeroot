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

//! Worker
//! do pow work

use crate::error;
use crate::work_manager::{WorkManager, Work};
use yee_primitives::{Config, Shard};
use std::sync::Arc;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use log::warn;
use runtime_primitives::traits::{Hash as HashT};
use parity_codec::{Decode, Encode};
use yee_consensus_pow_primitives::PowTarget;
use log::debug;
use parking_lot::RwLock;

const NONCE_STEPS : u64 = 100000;

pub struct Worker<WM: WorkManager> {
	config: Config,
	work_manager: Arc<RwLock<WM>>,
}

impl<WM> Worker<WM> where
	WM: WorkManager + Send + Sync + 'static,
	<WM::Hashing as HashT>::Output: Decode + Encode,
{
	pub fn new(config: Config, work_manager: Arc<RwLock<WM>>) -> Self{
		Self{
			config,
			work_manager,
		}
	}

	pub fn start(&self) -> error::Result<()> {

		let work_manager = self.work_manager.clone();

		let _thread = thread::Builder::new().name("miner".to_string()).spawn(move || {

			let mut old_work : Option<Work<<WM::Hashing as HashT>::Output>> = None;
			let mut old_nonce_start = 0u64;

			loop {
				let work = work_manager.read().get_work();
				match work {
					Err(e) => {
						warn!("miner error: {:?}", e);
						sleep(Duration::from_secs(3));
						continue;
					},
					Ok(work) => {
						debug!("New work: {:?}", work);
						let nonce_start = if Some(&work.merkle_root) == old_work.as_ref().map(|x|&x.merkle_root) {
							old_nonce_start
						}else{
							0u64
						};

						if nonce_start > u64::max_value() - NONCE_STEPS {
							warn!("miner error: nonce start too large");
							sleep(Duration::from_secs(3));
							continue;
						}
						let nonce_end = nonce_start + NONCE_STEPS;

						for nonce in nonce_start..nonce_end{

							//test hash
							let source = (work.merkle_root.clone(), work.extra_data.clone(), nonce);
							let source_hash = WM::Hashing::hash_of(&source);
							let source_pow_target = PowTarget::from(source_hash.as_ref());
							if &source_pow_target <= &work.target {
								let mut work_result = work.clone();
								work_result.nonce = Some(nonce);
								work_result.nonce_target = Some(source_pow_target);
								work_manager.write().submit_work(work_result);
								break;
							}
						}

						old_nonce_start = nonce_end;
						old_work = Some(work);

					}
				}
			}
		});

		Ok(())
	}

}