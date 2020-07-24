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

use std::{
    marker::{Send, Sync},
    sync::Arc,
};

use ansi_term::Colour;
use futures::{Stream, sync::mpsc};
use hash_db::Hasher;
use log::{debug, error, info, warn};
use parity_codec::{Compact, Decode, Encode};
use parking_lot::RwLock;
use runtime_primitives::{
    generic::BlockId,
    traits::{Block as BlockT, Digest, Hash, Header, ProvideRuntimeApi, Zero},
};
use substrate_cli::error;
use substrate_client::{
    self,
    BlockBody,
    blockchain::HeaderBackend,
    BlockchainEvents,
    ChainHead,
};
use substrate_primitives::{Blake2Hasher, H256, hexdisplay::HexDisplay};
use substrate_service::{
    config::Configuration,
    FactoryBlock,
    ServiceFactory,
    TaskExecutor,
};
use transaction_pool::txpool::{self, Pool as TransactionPool};

// use util::relay_decode::RelayTransfer;
use finality_tracker::FinalityTrackerDigestItem;
use foreign_chain::{ForeignChain, ForeignChainConfig};
use foreign_network::{message::generic::OutMessage, SyncProvider};
use yee_assets::Call as AssetsCall;
use yee_balances::Call as BalancesCall;
use yee_merkle::{MultiLayerProof, ProofAlgorithm, ProofHash};
use yee_primitives::RecommitRelay;
use yee_relay::Call as RelayCall;
use yee_runtime::{
    AccountId,
    Call,
    Hash as RuntimeHash,
    UncheckedExtrinsic,
};
use yee_sharding_primitives::ShardingAPI;
use yee_sr_primitives::{RelayParams, RelayTypes};

pub fn start_relay_transfer<F, C, A>(
    client: Arc<C>,
    executor: &TaskExecutor,
    foreign_network: Arc<dyn SyncProvider<FactoryBlock<F>, <FactoryBlock<F> as BlockT>::Hash>>,
    foreign_chains: Arc<RwLock<Option<ForeignChain<F>>>>,
    pool: Arc<TransactionPool<A>>,
    recomit_relay_receiver: mpsc::UnboundedReceiver<RecommitRelay<<F::Block as BlockT>::Hash>>,
) -> error::Result<()>
    where F: ServiceFactory + Send + Sync,
          C: 'static + Send + Sync,
          C: HeaderBackend<FactoryBlock<F>> + BlockBody<FactoryBlock<F>>,
          C: BlockchainEvents<FactoryBlock<F>> + ChainHead<<F as ServiceFactory>::Block>,
          C: ProvideRuntimeApi,
          <C as ProvideRuntimeApi>::Api: ShardingAPI<FactoryBlock<F>>,
          A: txpool::ChainApi + 'static,
          <F as ServiceFactory>::Configuration: ForeignChainConfig + Send + Sync,
          Configuration<<F as ServiceFactory>::Configuration, <F as ServiceFactory>::Genesis>: Clone,
          <<<F as ServiceFactory>::Block as BlockT>::Header as Header>::Number: From<u64>,
          <<<<F as ServiceFactory>::Block as BlockT>::Header as Header>::Digest as Digest>::Item: FinalityTrackerDigestItem,
          u64: From<<<<F as ServiceFactory>::Block as BlockT>::Header as Header>::Number>,
{
    // let network_send = foreign_network.clone();
    let network_rev = foreign_network.clone();
    let recommit_network = foreign_network.clone();
    let client_notify = client.clone();
    let client_recommit = client.clone();
    // let client_fe = client.clone();
    let import_events = client_notify.import_notification_stream()
        .for_each(move |notification| {
            let hash = notification.hash;
            let block_id = BlockId::Hash(hash);
            if let Ok(Some(header)) = client_notify.header(block_id) {
                if let Ok(Some(body)) = client_notify.block_body(&block_id) {
                    let api = client_notify.runtime_api();
                    let tc = api.get_shard_count(&block_id).expect("can't get shard count");    // total count
                    if let Ok(Some(cs)) = api.get_curr_shard(&block_id) {
                        for tx in &body {
                            let ec = tx.encode();
                            process_relay_extrinsic(ec, &header, hash, foreign_network.clone(), tc as u16, cs as u16);
                        }
                    }
                }
            }

            Ok(())
        });

    let foreign_events = network_rev.out_messages().for_each(move |messages| {
        match messages {
            OutMessage::RelayExtrinsics(txs) => {
                for tx in &txs {
                    let tx = tx.encode();
                    if let Some(_r_t) = RelayParams::<RuntimeHash>::decode(tx.clone()) {
                        let block_id = BlockId::number(Zero::zero());
                        if let Some(tx) = Decode::decode(&mut tx.as_slice()) {
                            let _ = pool.submit_relay_extrinsic(&block_id, tx).map_err(|e| warn!("submit relay extrinsic to pool failed: {:?}", e));
                        }
                    } else {
                        warn!(target: "foreign-relay", "receive bad relay extrinsic: {:?}", tx);
                    }
                }
                info!(target: "foreign-relay", "{}: {:?}", Colour::Green.paint("Receive relay-transaction"), txs);
            }
            OutMessage::BestBlockInfoChanged(shard_num, info) => {
                let number: u64 = info.best_number.into();
                if let Some(chain) = foreign_chains.read().as_ref().unwrap().get_shard_component(shard_num) {
                    let block_id = BlockId::number(number.into());
                    if let Ok(Some(spv_header)) = chain.client().header(&block_id) {
                        if let Some(finality_num) = spv_header.digest().logs().iter().rev()
                            .filter_map(FinalityTrackerDigestItem::as_finality_tracker)
                            .next() {
                            let block_id = BlockId::number(finality_num.into());
                            if let Ok(Some(crfg_header)) = chain.client().header(&block_id) {
                                let crfg_height: u64 = (*crfg_header.number()).into();
                                let tags = pool.relay_tags();
                                let mut ok_tags = Vec::with_capacity(tags.len());
                                for (shard, height, h, p_h) in tags {
                                    if shard.0 != shard_num || height.0 > crfg_height {
                                        continue;
                                    }
                                    let tag = (shard, height, h, p_h).encode();
                                    ok_tags.push(tag);
                                }
                                if ok_tags.len() > 0 {
                                    info!(target: "foreign-relay", "{}. shard_num: {}, number: {}", Colour::Green.bold().paint("CRFG reached"), shard_num, finality_num);
                                    pool.import_provides(ok_tags);
                                }
                            }
                        } else {
                            error!(target: "foreign-relay", "Can't get finality-tracker log. shard number:{}, number: {}!", shard_num, number);
                        }
                    }
                } else {
                    error!(target: "foreign-relay", "Get shard component({:?}) failed!", shard_num);
                }
            }
            _ => { /* do nothing */ }
        }
        Ok(())
    });

    let recommit_relay = recomit_relay_receiver.for_each(move |tx| {
        let hash = tx.hash;
        let block_id = BlockId::Hash(hash.into());
        let header = match client_recommit.header(block_id.clone()) {
            Ok(Some(h)) => h,
            _ => return Ok(())
        };
        let body = match client_recommit.block_body(&block_id) {
            Ok(Some(b)) => b,
            _ => return Ok(())
        };
        if tx.index >= body.len() {
            return Ok(());
        }
        let ec = body[tx.index].encode();

        let api = client_recommit.runtime_api();
        let tc = api.get_shard_count(&block_id).expect("can't get shard count");    // total count
        if let Ok(Some(cs)) = api.get_curr_shard(&block_id) {
            let ec = tx.encode();
            process_relay_extrinsic(ec, &header, hash, recommit_network.clone(), tc as u16, cs as u16);
        }
        Ok(())
    });

    executor.spawn(import_events);
    executor.spawn(foreign_events);
    executor.spawn(recommit_relay);
    Ok(())
}


fn process_relay_extrinsic<Block>(ec: Vec<u8>, header: &<Block as BlockT>::Header, hash: <<Block as BlockT>::Header as Header>::Hash, network_send: Arc<dyn SyncProvider<Block, <<Block as BlockT>::Header as Header>::Hash>>, tc: u16, cs: u16) -> bool where
    Block: BlockT<Hash=H256>,
    <<Block as BlockT>::Header as Header>::Number: From<u64>,
    u64: From<<<Block as BlockT>::Header as Header>::Number>,
{
    let mut result = false;
    debug!(target: "foreign-relay", "len: {}, origin: {}", &ec.len(), HexDisplay::from(&ec));
    let ex: UncheckedExtrinsic = match Decode::decode(&mut ec.as_slice()) {
        Some(v) => v,
        None => return false
    };
    match &ex.signature {
        None => return result,
        _ => {}
    }
    match ex.function {
        Call::Balances(BalancesCall::transfer(dest, value)) => {
            let ds = match yee_sharding_primitives::utils::shard_num_for(&dest, tc) {
                Some(v) => v,
                None => return result
            };
            if cs == ds {
                return result;
            }

            // create relay transfer
            let h: Compact<u64> = Compact((*header.number()).into());
            let function = Call::Relay(RelayCall::transfer(RelayTypes::Balance, ec, h, hash, *header.parent_hash()));
            let relay = UncheckedExtrinsic::new_unsigned(function);
            let buf = relay.encode();
            if let Some(relay) = Decode::decode(&mut buf.as_slice()){
                let relay_hash = <<Block as BlockT>::Header as Header>::Hashing::hash(buf.as_slice());
                info!(target: "foreign-relay", "{}: shard: {}, height: {}, amount: {}, hash:{:?}, encode: {}",
                      Colour::Green.paint("Send Balance-relay-transaction"), ds, h.0, value, relay_hash, HexDisplay::from(&buf));

                // broadcast relay transfer
                network_send.on_relay_extrinsics(ds, vec![(relay_hash, relay)]);
            };

        }
        Call::Assets(AssetsCall::transfer(_shard_code, id, dest, value)) => {
            let ds = match yee_sharding_primitives::utils::shard_num_for(&dest, tc as u16){
                Some(v) => v,
                None => return result
            };
            if cs == ds {
                return result;
            }

            // create relay transfer
            let h: Compact<u64> = Compact((*header.number()).into());
            let function = Call::Relay(RelayCall::transfer(RelayTypes::Assets, ec, h, hash, *header.parent_hash()));
            let relay = UncheckedExtrinsic::new_unsigned(function);
            let buf = relay.encode();
            if let Some(relay) = Decode::decode(&mut buf.as_slice()) {
                let relay_hash = <<Block as BlockT>::Header as Header>::Hashing::hash(buf.as_slice());
                info!(target: "foreign-relay", "{}: shard: {}, height: {}, AssetID: {}, amount: {}, hash:{:?}, encode: {}",
                      Colour::Green.paint("Send Asset-relay-transaction"), ds, h.0, id, value, relay_hash, HexDisplay::from(&buf));

                // broadcast relay transfer
                network_send.on_relay_extrinsics(ds, vec![(relay_hash, relay)]);
            }
        }
        _ => {}
    }
    return true;
}