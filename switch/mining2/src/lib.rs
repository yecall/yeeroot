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

pub mod error;
pub mod work_manager;
mod worker;
use yee_primitives::{Config, Shard};
use runtime_primitives::traits::{BlakeTwo256, Hash as HashT};
use std::sync::Arc;
use crate::work_manager::{WorkManager, DefaultWorkManager};
use yee_runtime::{BlockNumber, AuthorityId};
use parity_codec::{Decode, Encode};
use parking_lot::RwLock;

pub fn start_work_manager(config: &Config) -> error::Result<Arc<RwLock<DefaultWorkManager<
	BlockNumber,
	AuthorityId,
	BlakeTwo256>>>> {

	let mut work_manager = DefaultWorkManager::<
		BlockNumber,
		AuthorityId,
		BlakeTwo256>::new(config.clone());

	let work_manager = Arc::new(RwLock::new(work_manager));

	work_manager.write().start()?;

	Ok(work_manager)
}

pub fn start_mining<WM>(work_manager: Arc<RwLock<WM>>, config: &Config) -> error::Result<()>
where WM: WorkManager + Send + Sync + 'static,
	  <WM::Hashing as HashT>::Output: Decode + Encode,
{

	let worker = worker::Worker::new(config.clone(), work_manager);

	worker.start()?;

	Ok(())
}
