use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use future::FutureResult;
use futures::{Async, Future, Poll, Stream};
use futures::future;
use futures::future::join_all;
use futures::Join4;
use jsonrpc_core::BoxFuture;
use log::{debug, error, info, warn};
use parity_codec::{Compact, Decode, Encode};
use parking_lot::RwLock;
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

struct ExtrinsicFlag {
    shard: u16,
    hash: Hash,
    number: u64,
    index: u32,
    flag: u8,
}

pub struct RelayRecommitManager {
    rpc_client: Arc<RpcClient>,
    from: HashMap<u16, u64>,
    // value: u16: shard, Hash: Block hash, u64: block number, u32: tx index, u8: flag
    cross_shard_txs: Arc<RwLock<HashMap<Hash, ExtrinsicFlag>>>,
    current_height: Arc<RwLock<HashMap<u16, u64>>>,
}

impl RelayRecommitManager {
    pub fn new(config: Config, from: HashMap<u16, u64>) -> Self {
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
        self.start_fetch_thread(rpc_client, tx);

        let cross_shard_txs = self.cross_shard_txs.clone();
        let current_height = self.current_height.clone();
        let rpc_client = self.rpc_client.clone();

        let (mut tx_relay, rx_relay): (UnboundedSender<(u16, Hash, u32)>, UnboundedReceiver<(u16, Hash, u32)>) = mpsc::unbounded_channel();

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
                                let flag = ExtrinsicFlag { shard, hash, number: block.block.header.number, index: i as u32, flag: 0b01u8 };
                                // cross shard origin extrinsic
                                let mut txs = cross_shard_txs.write();
                                let entry = txs.entry(h).or_insert(flag);
                                if entry.flag & 0b10u8 == 0b10u8 {  // prune
                                    txs.remove(&h);
                                } else {
                                    entry.flag |= 0b01u8;
                                }
                            }
                            (false, Some(h)) => {
                                let flag = ExtrinsicFlag { shard, hash, number: block.block.header.number, index: i as u32, flag: 0b10u8 };
                                // relay extrinsic
                                let mut txs = cross_shard_txs.write();
                                let entry = txs.entry(h).or_insert(flag);
                                if entry.flag & 0b01u8 == 0b01u8 {  // prune
                                    txs.remove(&h);
                                } else {
                                    entry.flag |= 0b10u8;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                // recommit
                let txs = cross_shard_txs.read();
                for (_, flag) in txs.iter() {
                    let s = flag.shard;
                    let b_hash = flag.hash.clone();
                    let num = flag.number;
                    let index = flag.index;
                    if s == shard && block.block.header.number > num && block.block.header.number - num > 10 {
                        tx_relay.try_send((shard, b_hash, index));
                    }
                }
                Ok(())
            }).map_err(|_e| {});

            let fu_relay = rx_relay.for_each(move |(shard, b_hash, index)| {
                rpc_client.call_method_async::<_,jsonrpc_core::Error>("author_recommitRelay", "()", (b_hash, index), shard)
                    .unwrap_or_else(|e| Box::new(future::err(e.into()))).map_err(|e| { warn!("{:?}", e); });
                Ok(())
            }).map_err(|e| { warn!("{:?}", e); });
            rt.spawn(fu_relay);

            let _ = rt.block_on(rx_fu);
        });
    }

    fn start_fetch_thread(&self, rpc_client: Arc<RpcClient>, tx: UnboundedSender<(u16, Hash, RpcBlockResponse)>) {
        for i in 0..4 {
            let from = self.from.clone();
            let rpc_client_tmp = rpc_client.clone();
            let tx_tmp = tx.clone();
            let _ = std::thread::Builder::new().name(format!("fetch-shard#{}-block", i)).spawn(move || {
                let shard = i as u16;
                start_single_shard_fetch_thread(rpc_client_tmp.clone(), shard, from.get(&shard).cloned(), tx_tmp);
            });
        }
    }
}

fn start_single_shard_fetch_thread(rpc_client: Arc<RpcClient>, shard: u16, from: Option<u64>, mut tx: UnboundedSender<(u16, Hash, RpcBlockResponse)>) {
    let mut rt = Runtime::new().expect("can't start start_single_shard_fetch_thread");
    let mut current = from.unwrap_or(0u64);
    let mut latest = current;
    loop {
        let rpc_client_tmp = rpc_client.clone();
        let block = get_finalized_block_future(rpc_client_tmp, shard);
        match rt.block_on(block) {
            Ok(Ok(Some(v))) => {
                let v: RpcBlockResponse = v;
                if v.block.header.number > current {
                    latest = v.block.header.number;
                    if current == 0u64 {
                        current = latest;
                    }
                } else {
                    latest = current;
                }
            }
            _ => {
                info!("can't get finalized block");
            }
        };

        if current < latest {
            for num in current..latest + 1 {
                let rpc_client_tmp = rpc_client.clone();
                let block_fu = get_block_by_number_future(rpc_client_tmp, num, shard);
                let (block, hash) = match rt.block_on(block_fu) {
                    Ok(Ok((Some(v), hash))) => {
                        let v: RpcBlockResponse = v;
                        (v, hash)
                    }
                    _ => {
                        break;
                    }
                };
                let _ = tx.try_send((shard, hash, block));
                current = num;
            }
        }
        std::thread::sleep(Duration::from_secs(10));
    }
}

fn get_finalized_block_future(rpc_client: Arc<RpcClient>, shard: u16) -> BoxFuture<jsonrpc_core::Result<Option<RpcBlockResponse>>> {
    // get block hash
    let tmp_rpc_client = rpc_client.clone();
    let get_finalized_hash = || -> BoxFuture<jsonrpc_core::Result<Option<Hash>>> {
        let result = get_finalized_future(tmp_rpc_client, shard);
        let result = result.map(|x| Ok(x));
        Box::new(result)
    };
    let get_finalized_hash = get_finalized_hash();

    // get block
    let tmp_rpc_client = rpc_client.clone();
    let get_block = move || -> BoxFuture<jsonrpc_core::Result<Option<RpcBlockResponse>>> {
        let result = get_finalized_hash.and_then(move |x| match x {
            Ok(Some(hash)) => {
                let result = get_block_future(tmp_rpc_client, hash, shard);
                let result = result.map(|x| -> jsonrpc_core::Result<Option<RpcBlockResponse>> {
                    Ok(x)
                });
                Box::new(result) as BoxFuture<jsonrpc_core::Result<Option<RpcBlockResponse>>>
            }
            Ok(None) => Box::new(future::ok(Ok(None))),
            Err(e) => Box::new(future::err(e)),
        });
        Box::new(result)
    };
    get_block()
}

pub fn get_block_by_number_future(rpc_client: Arc<RpcClient>, number: u64, shard: u16)
                                  -> BoxFuture<jsonrpc_core::Result<(Option<RpcBlockResponse>, Hash)>> {
    // get block hash
    let tmp_rpc_client = rpc_client.clone();
    let get_hash = || -> BoxFuture<jsonrpc_core::Result<Option<Hash>>> {
        let result = get_block_hash_future(tmp_rpc_client, number, shard);
        let result = result.map(|x| Ok(x));
        Box::new(result)
    };
    let get_block_hash = get_hash();

    // get block
    let tmp_rpc_client = rpc_client.clone();
    let get_block = move || -> BoxFuture<jsonrpc_core::Result<(Option<RpcBlockResponse>, Hash)>> {
        let result = get_block_hash.and_then(move |x| match x {
            Ok(Some(hash)) => {
                let result = get_block_future(tmp_rpc_client, hash.clone(), shard);
                let result = result.map(move |x| -> jsonrpc_core::Result<(Option<RpcBlockResponse>, Hash)> {
                    Ok((x, hash))
                });
                Box::new(result) as BoxFuture<jsonrpc_core::Result<(Option<RpcBlockResponse>, Hash)>>
            }
            Ok(None) => Box::new(future::ok(Ok((None, Default::default())))),
            Err(e) => Box::new(future::err(e)),
        });
        Box::new(result)
    };
    get_block()
}

pub fn get_block_future(
    rpc_client: Arc<RpcClient>,
    hash: Hash,
    shard_num: u16,
) -> Box<dyn Future<Item=Option<RpcBlockResponse>, Error=jsonrpc_core::Error> + Send> {
    let result: BoxFuture<Option<RpcBlockResponse>> = rpc_client
        .call_method_async("chain_getBlock", "", ((hash, )), shard_num)
        .unwrap_or_else(|e| Box::new(future::err(e.into())));
    Box::new(result)
}

pub fn get_block_hash_future(
    rpc_client: Arc<RpcClient>,
    number: u64,
    shard_num: u16,
) -> Box<dyn Future<Item=Option<Hash>, Error=jsonrpc_core::Error> + Send> {
    let result: BoxFuture<Option<Hash>> = rpc_client
        .call_method_async("chain_getBlockHash", "", (number, ), shard_num)
        .unwrap_or_else(|e| Box::new(future::err(e.into())));
    Box::new(result)
}

pub fn get_finalized_future(rpc_client: Arc<RpcClient>, shard: u16) -> Box<dyn Future<Item=Option<Hash>, Error=jsonrpc_core::Error> + Send> {
    let result: BoxFuture<Option<Hash>> = rpc_client
        .call_method_async("chain_getFinalizedHead", "", (), shard)
        .unwrap_or_else(|e| Box::new(future::err(e.into())));
    Box::new(result)
}