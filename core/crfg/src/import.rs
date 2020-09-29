// Copyright 2018-2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

use std::{sync::Arc, collections::HashMap};

use log::{debug, trace, info, warn};
use parity_codec::Encode;
use futures::sync::mpsc;
use parking_lot::{RwLockWriteGuard, RwLock};

use client::{blockchain, CallExecutor, Client};
use client::blockchain::HeaderBackend;
use client::backend::Backend;
use client::runtime_api::ApiExt;
use consensus_common::{
	BlockImport, Error as ConsensusError, ErrorKind as ConsensusErrorKind,
	ImportBlock, ImportResult, JustificationImport, well_known_cache_keys,
};
use fg_primitives::CrfgApi;
use runtime_primitives::Justification;
use runtime_primitives::generic::BlockId;
use runtime_primitives::traits::{
	Block as BlockT, DigestFor, DigestItemFor, DigestItem,
	Header as HeaderT, NumberFor, ProvideRuntimeApi, As
};
use substrate_primitives::{H256, ed25519, Blake2Hasher};

use crate::{Error, CommandOrError, NewAuthoritySet, VoterCommand};
use crate::authorities::{AuthoritySet, SharedAuthoritySet, DelayKind, PendingChange};
use crate::consensus_changes::{SharedConsensusChanges, SharedPendingSkip};
use crate::environment::{finalize_block, is_descendent_of};
use crate::justification::CrfgJustification;

use ed25519::Public as AuthorityId;
use crate::digest::{CrfgChangeDigestItem, CrfgForceChangeDigestItem, CrfgSkipDigestItem};
use runtime_primitives::traits::Digest;
use std::time;
use std::cell::Cell;
use fg_primitives::BLOCK_FINAL_LATENCY;

const DEFAULT_FINALIZE_BLOCK: u64 = 1;
const FINALIZE_TIMEOUT: time::Duration = time::Duration::from_secs(30);

/// A block-import handler for CRFG.
///
/// This scans each imported block for signals of changing authority set.
/// If the block being imported enacts an authority set change then:
/// - If the current authority set is still live: we import the block
/// - Otherwise, the block must include a valid justification.
///
/// When using CRFG, the block import worker should be using this block import
/// object.
pub struct CrfgBlockImport<B, E, Block: BlockT<Hash=H256>, RA, PRA> {
	inner: Arc<Client<B, E, Block, RA>>,
	authority_set: SharedAuthoritySet<Block::Hash, NumberFor<Block>>,
	send_voter_commands: mpsc::UnboundedSender<VoterCommand<Block::Hash, NumberFor<Block>>>,
	consensus_changes: SharedConsensusChanges<Block::Hash, NumberFor<Block>>,
	api: Arc<PRA>,
	validator: bool,
	finalize_status: Arc<RwLock<Option<(NumberFor<Block>, time::Instant)>>>,
	import_until: Option<NumberFor<Block>>,
	pending_skip: SharedPendingSkip<NumberFor<Block>>,
}

impl<B, E, Block: BlockT<Hash=H256>, RA, PRA> JustificationImport<Block>
	for CrfgBlockImport<B, E, Block, RA, PRA> where
		NumberFor<Block>: grandpa::BlockNumberOps,
		B: Backend<Block, Blake2Hasher> + 'static,
		E: CallExecutor<Block, Blake2Hasher> + 'static + Clone + Send + Sync,
		DigestFor<Block>: Encode,
		DigestItemFor<Block>: DigestItem<AuthorityId=AuthorityId>,
		RA: Send + Sync,
		PRA: ProvideRuntimeApi,
		PRA::Api: CrfgApi<Block>,
{
	type Error = ConsensusError;

	fn on_start(&self, link: &(dyn ::consensus_common::import_queue::Link<Block>)) {
		let chain_info = match self.inner.info() {
			Ok(info) => info.chain,
			_ => return,
		};

		// request justifications for all pending changes for which change blocks have already been imported
		let authorities = self.authority_set.inner().read();
		for pending_change in authorities.pending_changes() {
			if pending_change.delay_kind == DelayKind::Finalized &&
				pending_change.effective_number() > chain_info.finalized_number &&
				pending_change.effective_number() <= chain_info.best_number
			{
				/*
				let effective_block_hash = self.inner.best_containing(
					pending_change.canon_hash,
					Some(pending_change.effective_number()),
				);

				if let Ok(Some(hash)) = effective_block_hash {
					if let Ok(Some(header)) = self.inner.header(&BlockId::Hash(hash)) {
						if *header.number() == pending_change.effective_number() {
							link.request_justification(&header.hash(), *header.number());
						}
					}
				}*/

				// since the delay of crfg is always 0, effective_block_hash = pending_change.canon_hash
				// the above code can be replaced with the following code
				// to avoid using best_containing( which contains unimplemented for light client)
				link.request_justification(&pending_change.canon_hash, pending_change.canon_height, false);
			}
		}
	}

	fn on_tick(&self, link: &(dyn ::consensus_common::import_queue::Link<Block>)) {
		let info = match self.inner.info(){
			Ok(info) => info,
			Err(e) => {
				warn!(target: "afg", "Justification import encounters error on tick: {:?}", e);
				return;
			}
		};
		let best_number = info.chain.best_number;
		let finalized_number = info.chain.finalized_number;
		let (status, updated) = match self.finalize_status.read().as_ref(){
			Some((f, i)) => {
				if f != &finalized_number {
					((finalized_number, time::Instant::now()), true)
				}else{
					((f.clone(), i.clone()), false)
				}
			},
			None => ((finalized_number, time::Instant::now()), true)
		};
		if updated {
			*self.finalize_status.write() = Some(status);
		}

		if best_number - finalized_number > As::sa(BLOCK_FINAL_LATENCY + 2 ) && status.1.elapsed() > FINALIZE_TIMEOUT {
			info!(target: "afg", "Finalize stalls, finalized_number: {} elapsed: {:?}", finalized_number, status.1.elapsed());

			let authorities = self.authority_set.inner().read();
			for pending_change in authorities.pending_changes() {
				if pending_change.delay_kind == DelayKind::Finalized &&
					pending_change.effective_number() > finalized_number &&
					pending_change.effective_number() <= best_number
				{
					link.request_justification(&pending_change.canon_hash, pending_change.canon_height, true);
				}
			}
			//clear status to avoid request justification too often
			*self.finalize_status.write() = None;
		}

	}

	fn import_justification(
		&self,
		hash: Block::Hash,
		number: NumberFor<Block>,
		justification: Justification,
	) -> Result<(), Self::Error> {
		self.import_justification(hash, number, justification, false)
	}
}

#[derive(Debug)]
enum AppliedChanges<H, N> {
	Standard(bool), // true if the change is ready to be applied (i.e. it's a root)
	Forced(NewAuthoritySet<H, N>),
	None,
}

impl<H, N> AppliedChanges<H, N> {
	fn needs_justification(&self) -> bool {
		match *self {
			AppliedChanges::Standard(_) => true,
			AppliedChanges::Forced(_) | AppliedChanges::None => false,
		}
	}
}

struct PendingSetChanges<'a, Block: 'a + BlockT> {
	just_in_case: Option<(
		AuthoritySet<Block::Hash, NumberFor<Block>>,
		RwLockWriteGuard<'a, AuthoritySet<Block::Hash, NumberFor<Block>>>,
	)>,
	applied_changes: AppliedChanges<Block::Hash, NumberFor<Block>>,
	do_pause: bool,
}

impl<'a, Block: 'a + BlockT> PendingSetChanges<'a, Block> {
	// revert the pending set change explicitly.
	fn revert(self) { }

	fn defuse(mut self) -> (AppliedChanges<Block::Hash, NumberFor<Block>>, bool) {
		self.just_in_case = None;
		let applied_changes = ::std::mem::replace(&mut self.applied_changes, AppliedChanges::None);
		(applied_changes, self.do_pause)
	}
}

impl<'a, Block: 'a + BlockT> Drop for PendingSetChanges<'a, Block> {
	fn drop(&mut self) {
		if let Some((old_set, mut authorities)) = self.just_in_case.take() {
			*authorities = old_set;
		}
	}
}

impl<B, E, Block: BlockT<Hash=H256>, RA, PRA> CrfgBlockImport<B, E, Block, RA, PRA> where
	NumberFor<Block>: grandpa::BlockNumberOps,
	B: Backend<Block, Blake2Hasher> + 'static,
	E: CallExecutor<Block, Blake2Hasher> + 'static + Clone + Send + Sync,
	DigestFor<Block>: Encode,
	DigestItemFor<Block>: DigestItem<AuthorityId=AuthorityId>,
	DigestItemFor<Block>: CrfgChangeDigestItem<NumberFor<Block>>,
	DigestItemFor<Block>: CrfgForceChangeDigestItem<NumberFor<Block>>,
	DigestItemFor<Block>: CrfgSkipDigestItem<NumberFor<Block>>,
	RA: Send + Sync,
	PRA: ProvideRuntimeApi,
	PRA::Api: CrfgApi<Block>,
{
    fn aggregate_authorities(authorities: &Vec<(AuthorityId, u64)>) -> Vec<(AuthorityId, u64)> {
		let mut polymer: Vec<(AuthorityId, u64)> = Vec::new();

		for author in authorities {
			match polymer.iter().position(|x| x.0 == author.0){
				Some(pos) => {
					let reappear = polymer.get_mut(pos);
					reappear.unwrap().1 += 1;
				},
				None => {
					polymer.push((author.0.clone(), author.1));
				}
			}
		}

		polymer
	}

	// check for a new authority set change.
	fn check_new_change(&self, header: &Block::Header, hash: Block::Hash)
		-> Result<Option<PendingChange<Block::Hash, NumberFor<Block>>>, ConsensusError>
	{

		let digest = header.digest();

		// check for forced change.
		// never process force change
		/*
		let at = BlockId::hash(*header.parent_hash());
		let api = self.api.runtime_api();
		{
			let maybe_change : Result<_, String> = Ok(digest.logs().iter().filter_map(CrfgForceChangeDigestItem::as_force_change).next());

			match maybe_change {
				Err(e) => match api.has_api_with::<dyn CrfgApi<Block>, _>(&at, |v| v >= 2) {
					Err(e) => return Err(ConsensusErrorKind::ClientImport(e.to_string()).into()),
					Ok(true) => {
						// API version is high enough to support forced changes
						// but got error, so it is legitimate.
						return Err(ConsensusErrorKind::ClientImport(e.to_string()).into())
					},
					Ok(false) => {
						// API version isn't high enough to support forced changes
					},
				},
				Ok(None) => {},
				Ok(Some((median_last_finalized, change))) => return {
					let authors = Self::aggregate_authorities(&change.next_authorities);
					let pending_change = PendingChange {
						next_authorities: authors,
						delay: change.delay,
						canon_height: *header.number(),
						canon_hash: hash,
						delay_kind: DelayKind::Best { median_last_finalized },
					};
					debug!(target: "afg", "Force pending change: {:?}", pending_change);
					Ok(Some(pending_change))
				},
			}
		}
		*/

		// check normal scheduled change.
		{
			let maybe_change : Result<_, String> = Ok(digest.logs().iter().filter_map(CrfgChangeDigestItem::as_change).next());

			match maybe_change {
				Err(e) => Err(ConsensusErrorKind::ClientImport(e.to_string()).into()),
				Ok(Some(change)) => {
					let authors = Self::aggregate_authorities(&change.next_authorities);
					let pending_change = PendingChange {
						next_authorities: authors,
						delay: change.delay,
						canon_height: *header.number(),
						canon_hash: hash,
						delay_kind: DelayKind::Finalized,
					};
					debug!(target: "afg", "Standard pending change: {:?}", pending_change);

					Ok(Some(pending_change))
				}
				Ok(None) => Ok(None),
			}
		}
	}

	fn check_skip(&self, block: &mut ImportBlock<Block>, finalized_number: NumberFor<Block>)
						-> Result<Option<NumberFor<Block>>, ConsensusError>
	{

		let header = &block.header;

		let digest = header.digest();

		let maybe_skip : Result<_, String> = Ok(digest.logs().iter().filter_map(CrfgSkipDigestItem::as_skip).next());

		let maybe_skip = match maybe_skip {
			Err(e) => Err(ConsensusErrorKind::ClientImport(e.to_string()).into()),
			Ok(Some(skip_number)) => {
				debug!(target: "afg", "Skip: {:?}", skip_number);

				let mut pending_skip = self.pending_skip.lock();
				pending_skip.push(skip_number);

				Ok(Some(skip_number))
			}
			Ok(None) => Ok(None),
		};

		// update aux
		let mut pending_skip = self.pending_skip.lock();
		pending_skip.retain(|x| x>=&finalized_number );
		crate::aux_schema::update_pending_skip(&*pending_skip,
											   |insert| block.auxiliary.extend(
												   insert.iter().map(|(k, v)| (k.to_vec(), Some(v.to_vec()))
												   )));
		debug!(target: "afg", "Pending skip: {:?}", *pending_skip);

		maybe_skip
	}

	fn make_authorities_changes<'a>(&'a self, block: &mut ImportBlock<Block>, hash: Block::Hash)
		-> Result<PendingSetChanges<'a, Block>, ConsensusError>
	{
		// when we update the authorities, we need to hold the lock
		// until the block is written to prevent a race if we need to restore
		// the old authority set on error or panic.
		struct InnerGuard<'a, T: 'a> {
			old: Option<T>,
			guard: Option<RwLockWriteGuard<'a, T>>,
		}

		impl<'a, T: 'a> InnerGuard<'a, T> {
			fn as_mut(&mut self) -> &mut T {
				&mut **self.guard.as_mut().expect("only taken on deconstruction; qed")
			}

			fn set_old(&mut self, old: T) {
				if self.old.is_none() {
					// ignore "newer" old changes.
					self.old = Some(old);
				}
			}

			fn consume(mut self) -> Option<(T, RwLockWriteGuard<'a, T>)> {
				if let Some(old) = self.old.take() {
					Some((old, self.guard.take().expect("only taken on deconstruction; qed")))
				} else {
					None
				}
			}
		}

		impl<'a, T: 'a> Drop for InnerGuard<'a, T> {
			fn drop(&mut self) {
				if let (Some(mut guard), Some(old)) = (self.guard.take(), self.old.take()) {
					*guard = old;
				}
			}
		}

		let number = block.header.number().clone();
		let maybe_change = self.check_new_change(
			&block.header,
			hash,
		)?;

		// returns a function for checking whether a block is a descendent of another
		// consistent with querying client directly after importing the block.
		let parent_hash = *block.header.parent_hash();
		let is_descendent_of = is_descendent_of(&self.inner, Some((&hash, &parent_hash)));

		let mut guard = InnerGuard {
			guard: Some(self.authority_set.inner().write()),
			old: None,
		};

		// whether to pause the old authority set -- happens after import
		// of a forced change block.
		let mut do_pause = false;

		// add any pending changes.
		if let Some(change) = maybe_change {
			let old = guard.as_mut().clone();
			guard.set_old(old);

			if let DelayKind::Best { .. } = change.delay_kind {
				do_pause = true;
			}

			guard.as_mut().add_pending_change(
				change,
				&is_descendent_of,
			).map_err(|e| ConsensusError::from(ConsensusErrorKind::ClientImport(e.to_string())))?;
		}

		let applied_changes = {
			let forced_change_set = guard.as_mut().apply_forced_changes(hash, number, &is_descendent_of)
				.map_err(|e| ConsensusErrorKind::ClientImport(e.to_string()))
				.map_err(ConsensusError::from)?;

			if let Some((median_last_finalized_number, new_set)) = forced_change_set {
				let new_authorities = {
					let (set_id, new_authorities) = new_set.current();

					// we will use the median last finalized number as a hint
					// for the canon block the new authority set should start
					// with. we use the minimum between the median and the local
					// best finalized block.
					let best_finalized_number = self.inner.backend().blockchain().info()
						.map_err(|e| ConsensusErrorKind::ClientImport(e.to_string()))?
						.finalized_number;

					let canon_number = best_finalized_number.min(median_last_finalized_number);

					let canon_hash =
						self.inner.backend().blockchain().header(BlockId::Number(canon_number))
							.map_err(|e| ConsensusErrorKind::ClientImport(e.to_string()))?
							.expect("the given block number is less or equal than the current best finalized number; \
									 current best finalized number must exist in chain; qed.")
							.hash();

					NewAuthoritySet {
						canon_number,
						canon_hash,
						set_id,
						authorities: new_authorities.to_vec(),
					}
				};
				let old = ::std::mem::replace(guard.as_mut(), new_set);
				guard.set_old(old);

				AppliedChanges::Forced(new_authorities)
			} else {
				let did_standard = guard.as_mut().enacts_standard_change(hash, number, &is_descendent_of)
					.map_err(|e| ConsensusErrorKind::ClientImport(e.to_string()))
					.map_err(ConsensusError::from)?;

				if let Some(root) = did_standard {
					AppliedChanges::Standard(root)
				} else {
					AppliedChanges::None
				}
			}
		};

		// consume the guard safely and write necessary changes.
		let just_in_case = guard.consume();
		if let Some((_, ref authorities)) = just_in_case {
			let authorities_change = match applied_changes {
				AppliedChanges::Forced(ref new) => {
					Some(new)
				},
				AppliedChanges::Standard(_) => {
					None // the change isn't actually applied yet.
				},
				AppliedChanges::None => {
					None
				},
			};

			crate::aux_schema::update_authority_set(
				authorities,
				authorities_change,
				|insert| block.auxiliary.extend(
					insert.iter().map(|(k, v)| (k.to_vec(), Some(v.to_vec())))
				)
			);
		}

		Ok(PendingSetChanges { just_in_case, applied_changes, do_pause })
	}
}

impl<B, E, Block: BlockT<Hash=H256>, RA, PRA> BlockImport<Block>
	for CrfgBlockImport<B, E, Block, RA, PRA> where
		NumberFor<Block>: grandpa::BlockNumberOps,
		B: Backend<Block, Blake2Hasher> + 'static,
		E: CallExecutor<Block, Blake2Hasher> + 'static + Clone + Send + Sync,
		DigestFor<Block>: Encode,
		DigestItemFor<Block>: DigestItem<AuthorityId=AuthorityId>,
		DigestItemFor<Block>: CrfgChangeDigestItem<NumberFor<Block>>,
		DigestItemFor<Block>: CrfgForceChangeDigestItem<NumberFor<Block>>,
		DigestItemFor<Block>: CrfgSkipDigestItem<NumberFor<Block>>,
		RA: Send + Sync,
		PRA: ProvideRuntimeApi,
		PRA::Api: CrfgApi<Block>,
{
	type Error = ConsensusError;

	fn import_block(&self, mut block: ImportBlock<Block>, new_cache: HashMap<well_known_cache_keys::Id, Vec<u8>>)
		-> Result<ImportResult, Self::Error>
	{
		let hash = block.post_header().hash();
		let number = block.header.number().clone();

		debug!(target: "afg", "Import block, number: {}, hash: {}, origin: {:?}",
			   number, hash, block.origin);

		if let Some(import_until) = self.import_until {
			if number > import_until {
				return Err(ConsensusErrorKind::ClientImport("Exceed max block number".to_string()).into());
			}
		}

		// early exit if block already in chain, otherwise the check for
		// authority changes will error when trying to re-import a change block
		match self.inner.backend().blockchain().status(BlockId::Hash(hash)) {
			Ok(blockchain::BlockStatus::InChain) => return Ok(ImportResult::AlreadyInChain),
			Ok(blockchain::BlockStatus::Unknown) => {},
			Err(e) => return Err(ConsensusErrorKind::ClientImport(e.to_string()).into()),
		}

		let chain_info = match self.inner.info() {
			Ok(info) => info.chain,
			Err(e) => return Err(ConsensusErrorKind::ClientImport(e.to_string()).into()),
		};
		let finalized_number = chain_info.finalized_number;

		debug!(target: "afg", "Check skip, number: {}, hash: {}, finalized_number: {}", number, hash, finalized_number);

		let skip = self.check_skip(&mut block, finalized_number)?;
		let pending_changes = self.make_authorities_changes(&mut block, hash)?;

		// we don't want to finalize on `inner.import_block`
		let mut justification = block.justification.take();
		let enacts_consensus_change = !new_cache.is_empty();
		let import_result = self.inner.import_block(block, new_cache);

		let mut imported_aux = {
			match import_result {
				Ok(ImportResult::Imported(aux)) => aux,
				Ok(r) => {
					debug!(target: "afg", "Restoring old authority set after block import result: {:?}", r);
					pending_changes.revert();
					return Ok(r);
				},
				Err(e) => {
					debug!(target: "afg", "Restoring old authority set after block import error: {:?}", e);
					pending_changes.revert();
					return Err(ConsensusErrorKind::ClientImport(e.to_string()).into());
				},
			}
		};

		let (applied_changes, do_pause) = pending_changes.defuse();

		debug!(target: "afg", "Applied changes, number: {}, hash: {}, applied_changes: {:?}", number, hash, applied_changes);

		// Send the pause signal after import but BEFORE sending a `ChangeAuthorities` message.
		if do_pause {
			if self.validator {
				let _ = self.send_voter_commands.unbounded_send(
					VoterCommand::Pause(format!("Forced change scheduled after inactivity"))
				);
			}
		}

		let needs_justification = applied_changes.needs_justification();

		match applied_changes {
			AppliedChanges::Forced(new) => {
				// NOTE: when we do a force change we are "discrediting" the old set so we
				// ignore any justifications from them. this block may contain a justification
				// which should be checked and imported below against the new authority
				// triggered by this forced change. the new crfg voter will start at the
				// last median finalized block (which is before the block that enacts the
				// change), full nodes syncing the chain will not be able to successfully
				// import justifications for those blocks since their local authority set view
				// is still of the set before the forced change was enacted, still after #1867
				// they should import the block and discard the justification, and they will
				// then request a justification from sync if it's necessary (which they should
				// then be able to successfully validate).
				if self.validator {
					let _ = self.send_voter_commands.unbounded_send(VoterCommand::ChangeAuthorities(new));
				}

				// we must clear all pending justifications requests, presumably they won't be
				// finalized hence why this forced changes was triggered
				imported_aux.clear_justification_requests = true;
			},
			AppliedChanges::Standard(false) => {
				// we can't apply this change yet since there are other dependent changes that we
				// need to apply first, drop any justification that might have been provided with
				// the block to make sure we request them from `sync` which will ensure they'll be
				// applied in-order.
				justification.take();
			},
			_ => {},
		}

		if !needs_justification && !enacts_consensus_change {
			return Ok(ImportResult::Imported(imported_aux));
		}

		if finalized_number + As::sa(srml_finality_tracker::STALL_LATENCY) <= number {
			if self.pending_skip.lock().contains(&finalized_number) {
				let next_number = finalized_number + As::sa(1);
				let next_hash = match self.inner.hash(next_number){
					Ok(Some(hash)) => hash,
					Ok(None) => return Err(ConsensusErrorKind::ClientImport("Unknown block number".to_string()).into()),
					Err(e) => return Err(ConsensusErrorKind::ClientImport(e.to_string()).into()),
				};
				debug!(target: "afg", "Execute skip, next_number: {}, next_hash: {}", &next_number, next_hash);
				match self.skip(next_hash, next_number){
					Ok(_) => (),
					Err(e) => return Err(ConsensusErrorKind::ClientImport(e.to_string()).into()),
				}
			}
		}

		debug!(target: "afg", "Import justification, number: {}, hash: {}, has_justification: {}", number, hash, justification.is_some());

		match justification {
			Some(justification) => {
				self.import_justification(hash, number, justification, needs_justification).unwrap_or_else(|err| {
					debug!(target: "afg", "Imported block #{} that enacts authority set change with \
						invalid justification: {:?}, requesting justification from peers.", number, err);
					imported_aux.bad_justification = true;
					imported_aux.needs_justification = true;
				});
			},
			None => {
				use runtime_primitives::traits::As;
				if number.as_() == DEFAULT_FINALIZE_BLOCK {
					let justification=  CrfgJustification::<Block>::default_justification(hash, number);
					self.import_justification(hash, number, justification.encode(), needs_justification).unwrap_or_else(|err| {
						debug!(target: "afg", "Imported block #{} that enacts authority set change with \
						invalid justification: {:?}, requesting justification from peers.", number, err);
						imported_aux.bad_justification = true;
						imported_aux.needs_justification = true;
					});
					return Ok(ImportResult::Imported(imported_aux))
				}

				if needs_justification {
					trace!(
						target: "afg",
						"Imported unjustified block #{} that enacts authority set change, waiting for finality for enactment.",
						number,
					);
				}

				// we have imported block with consensus data changes, but without justification
				// => remember to create justification when next block will be finalized
				if enacts_consensus_change {
					self.consensus_changes.lock().note_change((number, hash));
				}

				imported_aux.needs_justification = true;
			}
		}

		Ok(ImportResult::Imported(imported_aux))
	}

	fn check_block(
		&self,
		hash: Block::Hash,
		parent_hash: Block::Hash,
	) -> Result<ImportResult, Self::Error> {
		self.inner.check_block(hash, parent_hash)
	}
}

impl<B, E, Block: BlockT<Hash=H256>, RA, PRA> CrfgBlockImport<B, E, Block, RA, PRA> {
	pub(crate) fn new(
		inner: Arc<Client<B, E, Block, RA>>,
		authority_set: SharedAuthoritySet<Block::Hash, NumberFor<Block>>,
		send_voter_commands: mpsc::UnboundedSender<VoterCommand<Block::Hash, NumberFor<Block>>>,
		consensus_changes: SharedConsensusChanges<Block::Hash, NumberFor<Block>>,
		api: Arc<PRA>,
		validator: bool,
		import_until: Option<NumberFor<Block>>,
		pending_skip: SharedPendingSkip<NumberFor<Block>>,
	) -> CrfgBlockImport<B, E, Block, RA, PRA> {
		CrfgBlockImport {
			inner,
			authority_set,
			send_voter_commands,
			consensus_changes,
			api,
			validator,
			finalize_status: Arc::new(RwLock::new(None)),
			import_until,
			pending_skip,
		}
	}
}

impl<B, E, Block: BlockT<Hash=H256>, RA, PRA> CrfgBlockImport<B, E, Block, RA, PRA>
	where
		NumberFor<Block>: grandpa::BlockNumberOps,
		B: Backend<Block, Blake2Hasher> + 'static,
		E: CallExecutor<Block, Blake2Hasher> + 'static + Clone + Send + Sync,
		RA: Send + Sync,
{

	/// Import a block justification and finalize the block.
	///
	/// If `enacts_change` is set to true, then finalizing this block *must*
	/// enact an authority set change, the function will panic otherwise.
	fn import_justification(
		&self,
		hash: Block::Hash,
		number: NumberFor<Block>,
		justification: Justification,
		enacts_change: bool,
	) -> Result<(), ConsensusError> {
		let justification = CrfgJustification::decode_and_verify(
			justification,
			self.authority_set.set_id(),
			&self.authority_set.current_authorities(),
		);

		let justification = match justification {
			Err(e) => return Err(ConsensusErrorKind::ClientImport(e.to_string()).into()),
			Ok(justification) => justification,
		};

		let result = finalize_block(
			&*self.inner,
			&self.authority_set,
			&self.consensus_changes,
			None,
			hash,
			number,
			justification.into(),
			&self.pending_skip,
		);

		match result {
			Err(CommandOrError::VoterCommand(command)) => {
				debug!(target: "afg", "Imported justification for block #{} that triggers \
					command {}, signaling voter.", number, command);

				if self.validator {
					if let Err(e) = self.send_voter_commands.unbounded_send(command) {
						return Err(ConsensusErrorKind::ClientImport(e.to_string()).into());
					}
				}
			},
			Err(CommandOrError::Error(e)) => {
				return Err(match e {
					Error::Crfg(error) => ConsensusErrorKind::ClientImport(error.to_string()),
					Error::Network(error) => ConsensusErrorKind::ClientImport(error),
					Error::Blockchain(error) => ConsensusErrorKind::ClientImport(error),
					Error::Client(error) => ConsensusErrorKind::ClientImport(error.to_string()),
					Error::Safety(error) => ConsensusErrorKind::ClientImport(error),
					Error::Timer(error) => ConsensusErrorKind::ClientImport(error.to_string()),
				}.into());
			},
			Ok(_) => {
				assert!(!enacts_change, "returns Ok when no authority set change should be enacted; qed;");
			},
		}

		Ok(())
	}

	fn skip(
		&self,
		hash: Block::Hash,
		number: NumberFor<Block>,
	) -> Result<(), ConsensusError> {

		let justification = CrfgJustification::default_justification(hash.clone(), number);

		let result = finalize_block(
			&*self.inner,
			&self.authority_set,
			&self.consensus_changes,
			None,
			hash,
			number,
			justification.into(),
			&self.pending_skip,
		);

		match result {
			Err(CommandOrError::VoterCommand(command)) => {
				debug!(target: "afg", "Skip for block #{} that triggers \
					command {}, signaling voter.", number, command);

				if self.validator {
					if let Err(e) = self.send_voter_commands.unbounded_send(command) {
						return Err(ConsensusErrorKind::ClientImport(e.to_string()).into());
					}
				}
			},
			Err(CommandOrError::Error(e)) => {
				return Err(match e {
					Error::Crfg(error) => ConsensusErrorKind::ClientImport(error.to_string()),
					Error::Network(error) => ConsensusErrorKind::ClientImport(error),
					Error::Blockchain(error) => ConsensusErrorKind::ClientImport(error),
					Error::Client(error) => ConsensusErrorKind::ClientImport(error.to_string()),
					Error::Safety(error) => ConsensusErrorKind::ClientImport(error),
					Error::Timer(error) => ConsensusErrorKind::ClientImport(error.to_string()),
				}.into());
			},
			Ok(_) => (),
		}

		Ok(())
	}
}
