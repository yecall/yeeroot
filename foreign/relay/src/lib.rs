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
    sync::Arc,
    marker::{Send, Sync},
    iter::once,
};
use parity_codec::{Decode, Encode, Compact};
use futures::Stream;
use substrate_service::{
    config::Configuration,
    ServiceFactory,
    TaskExecutor,
    FactoryBlock,
};
use yee_runtime::{
    Call,
    UncheckedExtrinsic,
    AccountId,
    Hash as RuntimeHash,
};
use runtime_primitives::{
    generic::{BlockId, DigestItem},
    traits::{ProvideRuntimeApi, Digest, Block as BlockT, Header, Hash, Zero},
};
use substrate_client::{
    self,
    BlockchainEvents,
    ChainHead,
    blockchain::HeaderBackend,
    BlockBody,
};
use substrate_primitives::{
    hexdisplay::HexDisplay,
};
use transaction_pool::txpool::{self, Pool as TransactionPool};
use log::{debug, info, warn, error};
use substrate_cli::error;
use yee_balances::Call as BalancesCall;
use yee_assets::Call as AssetsCall;
use yee_relay::Call as RelayCall;
use yee_sharding_primitives::ShardingAPI;
use foreign_network::{SyncProvider, message::generic::OutMessage};
use foreign_chain::{ForeignChain, ForeignChainConfig};
use parking_lot::RwLock;
use util::relay_decode::RelayTransfer;
use finality_tracker::FinalityTrackerDigestItem;
use yee_sr_primitives::{RelayTypes, RelayParams};
use ansi_term::Colour;

pub fn start_relay_transfer<F, C, A>(
    client: Arc<C>,
    executor: &TaskExecutor,
    foreign_network: Arc<dyn SyncProvider<FactoryBlock<F>, <FactoryBlock<F> as BlockT>::Hash>>,
    foreign_chains: Arc<RwLock<Option<ForeignChain<F>>>>,
    pool: Arc<TransactionPool<A>>,
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
    let network_send = foreign_network.clone();
    let network_rev = foreign_network.clone();
    let client_notify = client.clone();
    let client_rcv = client.clone();
    let import_events = client_notify.import_notification_stream()
        .for_each(move |notification| {
            let hash = notification.hash;
            let block_id = BlockId::Hash(hash);
            let header = client_notify.header(block_id).unwrap().unwrap();
            let body = client_notify.block_body(&block_id).unwrap().unwrap();
            let api = client_notify.runtime_api();
            let tc = api.get_shard_count(&block_id).unwrap();    // total count
            let cs = api.get_curr_shard(&block_id).unwrap().unwrap();    // current shard
            for tx in &body {
                let ec = tx.encode();
                debug!(target: "foreign-relay", "len: {}, origin: {}", &ec.len(), HexDisplay::from(&ec));
                let ex: UncheckedExtrinsic = Decode::decode(&mut ec.as_slice()).unwrap();
                let sig = &ex.signature;
                if let None = sig {
                    continue;
                }
                match ex.function {
                    Call::Balances(BalancesCall::transfer(dest, value)) => {
                        let ds = yee_sharding_primitives::utils::shard_num_for(&dest, tc as u16);    // dest shard
                        if ds.is_none() {
                            continue;
                        }
                        let ds = ds.unwrap();
                        if cs as u16 == ds {
                            continue;
                        }

                        // create relay transfer
                        let h: Compact<u64> = Compact((*header.number()).into());
                        let function = Call::Relay(RelayCall::transfer(RelayTypes::Balance, ec, h, hash, *header.parent_hash()));
                        let relay = UncheckedExtrinsic::new_unsigned(function);
                        let buf = relay.encode();
                        let relay = Decode::decode(&mut buf.as_slice()).unwrap();
                        let relay_hash = <<FactoryBlock<F> as BlockT>::Header as Header>::Hashing::hash(buf.as_slice());
                        info!(target: "foreign-relay", "{}: shard: {}, height: {}, amount: {}, hash:{:?}, encode: {}",
                              Colour::Green.paint("Send Balance-relay-transaction"), ds, h.0, value, relay_hash, HexDisplay::from(&buf));

                        // broadcast relay transfer
                        network_send.on_relay_extrinsics(ds, vec![(relay_hash, relay)]);
                    },
                    Call::Assets(AssetsCall::transfer(_shard_code, id, dest, value)) => {
                        let ds = yee_sharding_primitives::utils::shard_num_for(&dest, tc as u16);    // dest shard
                        if ds.is_none() {
                            continue;
                        }
                        let ds = ds.unwrap();
                        if cs as u16 == ds {
                            continue;
                        }

                        // create relay transfer
                        let h: Compact<u64> = Compact((*header.number()).into());
                        let function = Call::Relay(RelayCall::transfer(RelayTypes::Assets, ec, h, hash, *header.parent_hash()));
                        let relay = UncheckedExtrinsic::new_unsigned(function);
                        let buf = relay.encode();
                        let relay = Decode::decode(&mut buf.as_slice()).unwrap();
                        let relay_hash = <<FactoryBlock<F> as BlockT>::Header as Header>::Hashing::hash(buf.as_slice());
                        info!(target: "foreign-relay", "{}: shard: {}, height: {}, AssetID: {}, amount: {}, hash:{:?}, encode: {}",
                              Colour::Green.paint("Send Asset-relay-transaction"), ds, h.0, id, value, relay_hash, HexDisplay::from(&buf));

                        // broadcast relay transfer
                        network_send.on_relay_extrinsics(ds, vec![(relay_hash, relay)]);
                    },
                    _ => {}
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
                        let tx = Decode::decode(&mut tx.as_slice()).unwrap();
                        pool.submit_relay_extrinsic(&block_id, tx, true).expect("Submit relay transfer into pool failed!");
                    } else {
                        warn!(target:"foreign-relay", "receive bad relay extrinsic: {:?}", tx);
                    }
                }
                info!(target: "foreign-relay", "{}: {:?}", Colour::Green.paint("Receive relay-transaction"), txs);
            }
            OutMessage::BestBlockInfoChanged(shard_num, info) => {
                let mut number: u64 = info.best_number.into();
                if let Some(chain) = foreign_chains.read().as_ref().unwrap().get_shard_component(shard_num) {
                    let block_id = BlockId::number(number.into());
                    let spv_header = chain.client().header(&block_id).unwrap().unwrap();
                    pool.enforce_spv(shard_num, number, info.best_hash.clone().as_ref().to_vec(), spv_header.parent_hash().as_ref().to_vec());
                    if let Some(finality_num) = spv_header.digest().logs().iter().rev()
                        .filter_map(FinalityTrackerDigestItem::as_finality_tracker)
                        .next() {
                        let block_id = BlockId::number(finality_num.into());
                        let crfg_header = chain.client().header(&block_id).unwrap().unwrap();
                        let tag = (Compact(shard_num), Compact(finality_num), crfg_header.hash().as_ref().to_vec(), crfg_header.parent_hash().as_ref().to_vec()).encode();
                        debug!(target: "foreign-relay", "{}. shard_num: {}, number: {}", Colour::Green.bold().paint("CRFG reached"), shard_num, finality_num);
                        pool.import_provides(once(tag));
                    } else {
                        error!(target: "foreign-relay", "Can't get finality-tracker log. shard number:{}, number: {}!", shard_num, number);
                    }
                } else {
                    error!(target: "foreign-relay", "Get shard component({:?}) failed!", shard_num);
                }
            }
            _ => { /* do nothing */ }
        }
        Ok(())
    });

    executor.spawn(import_events);
    executor.spawn(foreign_events);
    Ok(())
}
