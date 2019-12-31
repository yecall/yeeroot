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

use substrate_service::{ServiceFactory, TaskExecutor, Arc, ComponentClient, Components};
use futures::Stream;
use substrate_client::{BlockchainEvents};
use yee_sharding::{ScaleOutPhaseDigestItem, ScaleOutPhase};
use runtime_primitives::traits::{Block as BlockT, Header as HeaderT, Digest as DigestT, DigestItemFor};
use crate::FactoryBlockNumber;
use log::info;
use yee_runtime::{AccountId, AuthorityId};
use sharding_primitives::utils::shard_num_for;
use crate::service::ScaleOut;
use consensus::{CompatibleDigestItem, PowSeal};

pub struct Params {
	pub authority_id: Option<AuthorityId>,
	pub coinbase: AccountId,
	pub shard_num: u16,
	pub shard_count: u16,
	pub scale_out: Option<ScaleOut>,
	pub trigger_exit: Arc<dyn consensus::TriggerExit>,
}

pub fn start_restarter<C>(param: Params, client: Arc<ComponentClient<C>>, executor: &TaskExecutor) where
	C: Components,
	<<C::Factory as ServiceFactory>::Block as BlockT>::Header: HeaderT,
	DigestItemFor<<C::Factory as ServiceFactory>::Block>: ScaleOutPhaseDigestItem<FactoryBlockNumber<C::Factory>, u16> + CompatibleDigestItem<<C::Factory as ServiceFactory>::Block, AuthorityId>,
{
	let target_shard_num = match param.scale_out.clone(){
		Some(scale_out) => scale_out.shard_num,
		None => param.shard_num,
	};

	let authority_id = param.authority_id.clone();

	let coinbase = param.coinbase.clone();
	let trigger_exit = param.trigger_exit;

	let task = client.import_notification_stream().for_each(move |notification| {

		let header = notification.header;

		let scale_out_phase : Option<ScaleOutPhase<FactoryBlockNumber<C::Factory>, u16>> = header.digest().logs().iter().rev()
			.filter_map(ScaleOutPhaseDigestItem::as_scale_out_phase)
			.next();

		if let Some(scale_out_phase) = scale_out_phase.clone(){
			info!("Scale out phase: {:?}", scale_out_phase);
		}

		if let Some(ScaleOutPhase::Committing {shard_count}) = scale_out_phase{

			let self_mined = match authority_id {
				Some(ref authority_id) => {
					let pow_seal : Option<PowSeal<<C::Factory as ServiceFactory>::Block, AuthorityId>> = header.digest().logs().iter().rev()
						.filter_map(CompatibleDigestItem::as_pow_seal)
						.next();
					match pow_seal{
						Some(pow_seal) => {
							authority_id.clone() == pow_seal.authority_id
						},
						None => false,
					}
				},
				None => false,
			};
			info!("Self mined: {}", self_mined);

			// if is self mined, will not stop or restart here, we need time to let the new block propagate
			if !self_mined {
				let coinbase_shard_num = shard_num_for(&coinbase, shard_count).expect("qed");
				if target_shard_num != coinbase_shard_num {
					info!("Stop service for coinbase shard num is not accordant");
					trigger_exit.trigger_stop();
					return Ok(());
				}

				info!("Restart service for commiting scale out phase");
				trigger_exit.trigger_restart();
			}

		}

		Ok(())
	});

	executor.spawn(task);

}
