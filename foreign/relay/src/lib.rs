use std::{
    sync::Arc,
    marker::{Send, Sync},
};
use parity_codec::{Decode, Encode, Compact};
use futures::{
    Stream,
    future::{self, Loop},
    Future, IntoFuture,
};
use substrate_service::{
    ServiceFactory,
    TaskExecutor,
    FactoryBlock,
    FactoryExtrinsic,
};
use yee_runtime::{
    Call,
    Block,
    UncheckedExtrinsic,
};
use runtime_primitives::{
    generic::{ BlockId, UncheckedMortalCompactExtrinsic },
    traits::{ ProvideRuntimeApi, Block as BlockT, Header },
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
    H256,
    Blake2Hasher,
    Hasher,
};
use pool_graph::{
    ChainApi,
    ExtrinsicFor,
};
use transaction_pool::txpool::{self, Pool as TransactionPool};
use log::{debug, info, warn};
use substrate_cli::error;
use yee_balances::Call as BalancesCall;
use yee_sharding_primitives::ShardingAPI;


pub fn start_relay_transfer<F, C, A>(
    client: Arc<C>,
    executor: &TaskExecutor,
    foreign_network: Box<Arc<network::SyncProvider<FactoryBlock<F>, H256>>>,
    pool: Arc<TransactionPool<A>>
) -> error::Result<()>
    where F: ServiceFactory,
          C: 'static + Send + Sync,
          C: HeaderBackend<FactoryBlock<F>> + BlockBody<FactoryBlock<F>>,
          C: BlockchainEvents<FactoryBlock<F>>,
          C: ProvideRuntimeApi,
          <C as ProvideRuntimeApi>::Api: ShardingAPI<FactoryBlock<F>>,
          A: txpool::ChainApi + 'static,
{
    let network_send = foreign_network.clone();
    let network_rev = foreign_network.clone();
    let import_events = client.import_notification_stream()
        .for_each(move |notification| {
            let hash = notification.hash;
            let blockId = BlockId::Hash(hash);
            let header = client.header(blockId).unwrap().unwrap();
            let body = client.block_body(&blockId).unwrap().unwrap();
            for mut tx in &body {
                let ec = tx.encode();
                debug!(target: "relay", "len: {}, origin: {}", &ec.len(), HexDisplay::from(&ec));
                let ex: UncheckedExtrinsic = Decode::decode(&mut ec.as_slice()).unwrap();
                let sig = &ex.signature;
                if let None = sig {
                    continue;
                }
                if let Call::Balances(BalancesCall::transfer(dest, value)) = &ex.function {
                    let api = client.runtime_api();
                    let tc = api.get_shard_count(&blockId).unwrap();    // total count
                    let cs = api.get_curr_shard(&blockId).unwrap().unwrap();    // current shard
                    let ds = yee_sharding_primitives::utils::shard_num_for(dest, tc as u16);    // dest shard
                    if ds.is_none() {
                        continue;
                    }
                    let ds = ds.unwrap();
                    if cs as u16 == ds {
                        continue;
                    }

                    // create relay transfer
                    let proof: Vec<u8> = vec![]; // todo
                    let h = header.number().encode();
                    let mut h = h.as_slice();
                    let h: Compact<u64> = Decode::decode(&mut h).unwrap();
                    let function = Call::Balances(BalancesCall::relay_transfer(ec, h, hash, *header.parent_hash(), proof));
                    let relay = UncheckedExtrinsic::new_unsigned(function);
                    let buf = relay.encode();
                    let relay = Decode::decode(&mut buf.as_slice()).unwrap();
                    let relay_hash = Blake2Hasher::hash(buf.as_slice());
                    info!(target: "relay", "shard: {}, amount: {}, hash:{}, encode: {}", ds, value, relay_hash, HexDisplay::from(&buf));

                    // broadcast relay transfer
                    network_send.on_relay_extrinsics(ds, vec![(relay_hash, relay)]);
                }
            }

            Ok(())
        });

    let foreign_events = network_rev.out_messages().for_each(move |messages| {
        match messages {
            network::message::generic::OutMessage::Extrinsics(txs) =>{
                let h = 0u64;
                let h = h.encode();
                let h=Decode::decode(&mut h.as_slice()).unwrap();
                let blockId = BlockId::number(h);
                for tx in txs{
                    let tx = tx.encode();
                    let tx = Decode::decode(&mut tx.as_slice()).unwrap();
                    pool.submit_one(&blockId, tx);
                }
                info!(target: "relay","received relay transaction: {:?}", tx);
            }
            _ =>{ /* do nothing */ }
        }
        Ok(())
    });

    executor.spawn(import_events);
    executor.spawn(foreign_events);
    Ok(())
}