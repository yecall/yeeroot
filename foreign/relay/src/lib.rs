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
    //opaque::UncheckedExtrinsic,
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

pub fn start_relay_transfer<F, C>(
    client: Arc<C>,
    executor: &TaskExecutor,
) -> error::Result<()>
    where F: ServiceFactory,
          C: 'static + Send + Sync,
          C: BlockBody<FactoryBlock<F>>,
          C: BlockchainEvents<FactoryBlock<F>>,
// FactoryBlock<F>: BlockT<Extrinsic=UncheckedExtrinsic>
{
    let events = client.import_notification_stream()
        .for_each(move |notification| {
            println!("zh new event");
            let hash = notification.hash;
            let body= client.block_body(&BlockId::Hash(hash)).unwrap().unwrap();
            for mut tx in &body {
                let ec = tx.encode();
                println!("{}",HexDisplay::from(&ec));
                // let ex: UncheckedMortalCompactExtrinsic<_,_,_,_> = Decode::decode(&mut tx.encode().as_slice()).unwrap();


////                    let (relay,t_n) = create_relay(tx);
////                    if relay.len() > 0 {
////                        // send relay transfer
////                        // todo
////                    }
//
//                    let mut tx = tx.clone();
//                    let ex = tx;
//                    //let ex = Decode::decode(&tx).unwrap();
//                    let sig = &ex.signature;
//                    // todo check signature
//
//                    if let Call::Balances(BalancesCall::transfer(dest, value)) = &ex.function {
//                        let c_n = 0;
//                        let t_c = 4;
//                        let t_n = yee_sharding_primitives::utils::shard_num_for(dest, t_c);
//                        if t_n.is_none() {
//                            //return (vec![], 0);
//                        }
//                        let t_n = t_n.unwrap();
//                        if c_n == t_n {
//                            //return (vec![], 0);
//                        }
//                        // create relay transfer
//                        // todo
//                        //return (vec![1], t_n);
//                    }
            }

            Ok(())
        });
    executor.spawn(events);
    Ok(())
}

//fn create_relay(ex: <Block as BlockT>::Extrinsic) -> (Vec<u8>, u16)
//{
//    let ex = Decode::decode(&ex).unwrap();
//    let sig = &ex.signature;
//    // todo check signature
//
//    if let Call::Balances(BalancesCall::transfer(dest, value)) = &ex.function {
//        let c_n = 0;
//        let t_c = 4;
//        let t_n = yee_sharding_primitives::utils::shard_num_for(dest, t_c);
//        if t_n.is_none() {
//            return (vec![], 0);
//        }
//        let t_n = t_n.unwrap();
//        if c_n == t_n {
//            return (vec![], 0);
//        }
//        // create relay transfer
//        // todo
//        return (vec![1], t_n);
//    }
//    (vec![], 0)
//}