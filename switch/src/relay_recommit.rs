use std::collections::HashMap;
// use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::{Async, Future, Poll, Stream};
use futures::future;
use futures::future::join_all;
use futures::Join4;
use jsonrpc_core::BoxFuture;
use log::{info, warn};
use parking_lot::RwLock;
use parity_codec::{Decode, Encode, Compact};
use serde_json::Value;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::timer::Interval;

use relay_monitor::types::{decode_extrinsic, RpcBlockResponse};
use yee_primitives::Config;
use yee_runtime::Hash;
use yee_switch_rpc::client::RpcClient;

use crate::error;

pub struct RelayRecommitManager {
    rpc_client: Arc<RpcClient>,
    from: u64,
    // value: u16: shard, Hash: Block hash, u64: block number, u32: tx index
    cross_shard_txs: Arc<RwLock<HashMap<Hash, (u16, Hash, u64, u32)>>>,
    current_height: Arc<RwLock<HashMap<u16, u64>>>,
}

impl RelayRecommitManager {
    pub fn new(config: Config, from: u64) -> Self {
        Self {
            rpc_client: Arc::new(RpcClient::new(config)),
            from,
            cross_shard_txs: Arc::new(RwLock::new(HashMap::new())),
            current_height: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn start(&self) {
        let (tx, rx): (UnboundedSender<(u16, Hash, RpcBlockResponse)>, UnboundedReceiver<(u16, Hash, RpcBlockResponse)>) = mpsc::unbounded_channel();
        let rpc_client = self.rpc_client.clone();

        let _ = std::thread::Builder::new().name("get-finalized".to_string()).spawn(move || {
            Self::get_finalized(rpc_client, tx);
        });
        let cross_shard_txs = self.cross_shard_txs.clone();
        let current_height = self.current_height.clone();
        let rpc_client = self.rpc_client.clone();
        let _ = std::thread::Builder::new().name("finalized-loop".to_string()).spawn(move || {
            let mut rt = Runtime::new().expect("can't start finalized-loop thread");
            let rx_fu = rx.for_each(move |(shard, hash, block)| {
                {
                    let mut current_height = current_height.write();
                    let entry = current_height.entry(shard).or_insert(block.block.header.number);
                    *entry = block.block.header.number;
                }
                if block.block.extrinsics.len() > 5 {
                    let txs = block.block.extrinsics;
                    for i in 5..txs.len() {
                        let tx = txs[i].clone();
                        let mut tx_arr = hex::decode(tx.trim_start_matches("0x")).expect("qed");
                        let mut length_prefix: Vec<u8> = Compact(tx_arr.len() as u32).encode();
                        length_prefix.append(&mut tx_arr);
                        match decode_extrinsic(length_prefix, 4u16, shard) {
                            (true, Some(h)) => {
                                // cross shard origin extrinsic
                                let mut txs = cross_shard_txs.write();
                                txs.entry(h).or_insert((shard, hash, block.block.header.number, i as u32));
                            }
                            (false, Some(h)) => {
                                // relay extrinsic
                                let mut txs = cross_shard_txs.write();
                                txs.remove(&h);
                            }
                            _ => {}
                        }
                    }
                }
                // recommit
                let txs = cross_shard_txs.read();
                for (_, (s, h, num, index)) in txs.iter() {
                    let s = *s;
                    let b_hash = h.clone();
                    let num = *num;
                    let index = *index;
                    if s == shard && num > block.block.header.number && num - block.block.header.number > 10 {
                        let f = rpc_client.call_method_async("author_recommitRelay", "()", (b_hash, index), shard)
                            .unwrap_or_else(|e| Box::new(future::err(e.into()))).map_err(|e| { warn!("{:?}", e); });
                        tokio::spawn(f);
                    }
                }
                Ok(())
            }).map_err(|_e| {});
            let _ = rt.block_on(rx_fu);
        });
    }

    fn get_finalized(rpc_client: Arc<RpcClient>, mut tx: UnboundedSender<(u16, Hash, RpcBlockResponse)>) {
        let mut finalized_info: HashMap<u16, Hash> = HashMap::new();
        let interval = Interval::new_interval(Duration::from_secs(10));
        let mut rt = Runtime::new().expect("can't start RelayRecommitManager");

        let task = interval.for_each(move |_| {
            let get_block_task = |rpc_client: Arc<RpcClient>, h: Hash, shard: u16| -> BoxFuture<Option<RpcBlockResponse>> {
                rpc_client.call_method_async("chain_getBlock", "Option<Value>", ((h, )), shard)
                    .unwrap_or_else(|e| Box::new(future::err(e.into())))
            };

            let f_0: BoxFuture<Option<Hash>> = rpc_client.call_method_async("chain_getFinalizedHead", "Option<Hash>", (), 0)
                .unwrap_or_else(|e| Box::new(future::err(e.into())));
            let f_1: BoxFuture<Option<Hash>> = rpc_client.call_method_async("chain_getFinalizedHead", "Option<Hash>", (), 1)
                .unwrap_or_else(|e| Box::new(future::err(e.into())));
            let f_2: BoxFuture<Option<Hash>> = rpc_client.call_method_async("chain_getFinalizedHead", "Option<Hash>", (), 2)
                .unwrap_or_else(|e| Box::new(future::err(e.into())));
            let f_3: BoxFuture<Option<Hash>> = rpc_client.call_method_async("chain_getFinalizedHead", "Option<Hash>", (), 3)
                .unwrap_or_else(|e| Box::new(future::err(e.into())));
            let pair = f_0.join4(f_1, f_2, f_3);
            match pair.wait() {
                Ok((v0, v1, v2, v3)) => {
                    let mut tasks: Vec<BoxFuture<Option<RpcBlockResponse>>> = Vec::with_capacity(4);
                    let mut flag: Vec<(u16, Hash)> = Vec::with_capacity(4);
                    v0.map(|h| {
                        let mut update = false;

                        if finalized_info.contains_key(&0) {
                            let s_0 = finalized_info.entry(0).or_default();
                            if *s_0 != h {
                                *s_0 = h;
                                update = true;
                            }
                        } else {
                            finalized_info.entry(0).or_insert(h);
                            update = true;
                        }

                        if update {
                            tasks.push(get_block_task(rpc_client.clone(), h, 0));
                            flag.push((0, h));
                        }
                    });

                    v1.map(|h| {
                        let mut update = false;
                        if finalized_info.contains_key(&1) {
                            let s_1 = finalized_info.entry(1).or_default();
                            if *s_1 != h {
                                *s_1 = h;
                                update = true;
                            }
                        } else {
                            finalized_info.entry(1).or_insert(h);
                            update = true;
                        }

                        if update {
                            tasks.push(get_block_task(rpc_client.clone(), h, 1));
                            flag.push((1, h));
                        }
                    });

                    v2.map(|h| {
                        let mut update = false;
                        if finalized_info.contains_key(&2) {
                            let s_2 = finalized_info.entry(2).or_default();
                            if *s_2 != h {
                                *s_2 = h;
                                update = true;
                            }
                        } else {
                            finalized_info.entry(2).or_insert(h);
                            update = true;
                        }

                        if update {
                            tasks.push(get_block_task(rpc_client.clone(), h, 2));
                            flag.push((2, h));
                        }
                    });

                    v3.map(|h| {
                        let mut update = false;
                        if finalized_info.contains_key(&3) {
                            let s_3 = finalized_info.entry(3).or_default();
                            if *s_3 != h {
                                *s_3 = h;
                                update = true;
                            }
                        } else {
                            finalized_info.entry(3).or_insert(h);
                            update = true;
                        }

                        if update {
                            tasks.push(get_block_task(rpc_client.clone(), h, 3));
                            flag.push((3, h));
                        }
                    });

                    if tasks.len() > 0 {
                        match join_all(tasks).wait() {
                            Ok(blocks) => {
                                let mut index = 0;
                                for b in blocks {
                                    tx = tx.clone();
                                    match b {
                                        Some(block) => {
                                            let f = flag[index];
                                            let _ = tx.try_send((f.0, f.1, block));
                                        }
                                        None => {}
                                    }
                                    index = index + 1;
                                }
                            }
                            Err(_e) => {}
                        }
                    }
                }
                Err(_) => {}
            }
            Ok(())
        }).map_err(|_| {});
        let _ = rt.block_on(task);
    }
}

