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
use substrate_cli::error;
use yee_balances::Call as BalancesCall;
use yee_sharding_primitives::ShardingAPI;
use yee_relay_primitives as relay_primitives;


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
                let ex: UncheckedExtrinsic = Decode::decode(&mut ec.as_slice()).unwrap();
                let sig = &ex.signature;
                if let None = sig {
                    continue;
                }
                // todo check signature

                if let Call::Balances(BalancesCall::transfer(dest, value)) = &ex.function {
                    let api = client.runtime_api();
                    let t_c = api.get_shard_count(&blockId).unwrap();
                    let c_n = api.get_curr_shard(&blockId).unwrap().unwrap();
                    let t_n = yee_sharding_primitives::utils::shard_num_for(dest, t_c as u16);
                    if t_n.is_none() {
                        continue;
                    }
                    let t_n = t_n.unwrap();
                    let c_n = c_n as u16;
                    if c_n == t_n {
                        continue;
                    }
                    // create relay transfer
                    // todo
                    println!("zh: {}", HexDisplay::from(&ec));
                    let relay = relay_primitives::RelayTransfer::new(ec, hash, vec![]);

                }
            }

            Ok(())
        });
    executor.spawn(events);
    Ok(())
}