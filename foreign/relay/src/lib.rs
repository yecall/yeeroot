use std::{
    sync::Arc,
    marker::{Send, Sync},
};
use parity_codec::{Decode, Encode};
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
    generic::{BlockId, UncheckedMortalCompactExtrinsic},
    traits::{ProvideRuntimeApi, Block as BlockT},
};
use substrate_client::{
    self,
    BlockchainEvents,
    ChainHead,
    BlockBody,
};
use substrate_primitives::{
    hexdisplay::HexDisplay,
};
use pool_graph::{
    ChainApi,
    ExtrinsicFor,
};
use log::{debug, info, warn};
use substrate_cli::error;
use yee_balances::Call as BalancesCall;
use yee_sharding_primitives::ShardingAPI;


pub fn start_relay_transfer<F, C>(
    client: Arc<C>,
    executor: &TaskExecutor,
) -> error::Result<()>
    where F: ServiceFactory,
          C: 'static + Send + Sync,
          C: BlockBody<FactoryBlock<F>>,
          C: BlockchainEvents<FactoryBlock<F>>,
          C: ProvideRuntimeApi + ChainHead<FactoryBlock<F>>,
          <C as ProvideRuntimeApi>::Api: ShardingAPI<FactoryBlock<F>>,
{
    let events = client.import_notification_stream()
        .for_each(move |notification| {
            let hash = notification.hash;
            let blockId = BlockId::Hash(hash);
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
                    let function = Call::Balances(BalancesCall::relay_transfer(ec, hash, proof));
                    let relay = UncheckedExtrinsic::new_unsigned(function);
                    let buf = relay.encode();
                    info!(target: "relay", "amount: {}, encode: {}", value, HexDisplay::from(&buf));

                    // broadcast relay transfer
                    // todo
                }
            }

            Ok(())
        });
    executor.spawn(events);
    Ok(())
}