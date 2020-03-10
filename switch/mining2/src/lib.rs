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

mod error;
pub mod work_manager;
mod worker;
use yee_primitives::{Config, Shard};
use runtime_primitives::traits::{BlakeTwo256};
use std::sync::Arc;

pub fn start_mining(config: &Config) -> error::Result<()>{

	let mut work_manager = work_manager::DefaultWorkManager::<
		yee_runtime::BlockNumber,
		yee_runtime::AuthorityId,
		BlakeTwo256>::new(config.clone());
	work_manager.start()?;

	let worker = worker::Worker::new(config.clone(), Arc::new(work_manager));

	worker.start()?;

	Ok(())
}
