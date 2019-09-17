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

use crate::client::{Client,RpcError,Rpc};
use crate::Work;
use crate::WorkMap;
use crossbeam_channel::{select, unbounded, Receiver};
use std::thread;
use log::{info,error,warn,debug};
use crate::job_template::{ProofMulti, JobTemplate, DifficultyType};
use yee_lru_cache::LruCache;
use yee_util::Mutex;
use std::time;
use hyper::rt::{self, Future, Stream};
use yee_jsonrpc_types::{
    error::Error as RpcFail, error::ErrorCode as RpcFailCode, id::Id, params::Params,
    request::MethodCall, response::Output, version::Version};
use crossbeam_channel::Sender;
use std::collections::HashMap;
use std::any::Any;
use failure::Error;
use uuid::Uuid;
use crate::merkle::{CryptoYeeAlgorithm,CryptoSHA256Hash,HexSlice};
use yee_merkle::hash::{Algorithm, Hashable};
use yee_merkle::merkle::MerkleTree;
use std::iter::FromIterator;
use primitives::{H256,blake2_256};

extern crate crypto;
use std::fmt;
use std::hash::Hasher;
use crypto::sha2::Sha256;
use crypto::digest::Digest;
use primitives::hexdisplay::HexDisplay;
use crate::job::Job;
use yee_switch_rpc::Config;
use rand::Rng;

const WORK_CACHE_SIZE: usize = 32;

pub struct Gateway {
    pub current_job_set:HashMap<String,JobTemplate>,
    pub client: Client,
    pub shard_job_cache: Mutex<LruCache<String,JobTemplate>>,
    pub new_work_tx: Sender<WorkMap>,
    pub map:Config,
}

impl Gateway {
    pub fn new(client: Client,new_work_tx: Sender<WorkMap>,map:Config) -> Gateway {
        //init

        let job = JobTemplate{ difficulty:  DifficultyType::from(0x00000000) << 224,
                               rawHash: blake2_256( "".as_bytes()).into(),
            url: "".to_string()
        };


        let mut  set:HashMap<String,JobTemplate> =  HashMap::new();

        for (key, value) in &map.shards {
            set.insert(key.to_string(), job.clone());
        }

        Gateway {
            current_job_set: set,
            client,
            shard_job_cache: Mutex::new(LruCache::new(WORK_CACHE_SIZE)),
            new_work_tx,
            map,
        }
    }

    pub fn poll_job_template(&mut self) {
        debug!("thsi is poll_job_template thread id {:?}",thread::current().id());
        loop {
            debug!("poll job template...");
            self.try_update_job_template();
            thread::sleep(time::Duration::from_millis(self.client.config.poll_interval));
        }
    }

    pub fn try_update_job_template(&mut self) {
        let mut  set:HashMap<String,JobTemplate> =  HashMap::new();

        for (key, value) in &self.map.shards {
           // debug!("node url---[{}] = {}", key, value);

            let rpc = &value.rpc;
            let mut rng =rand::thread_rng();
            let i = rng.gen_range(0, rpc.len());
            let mut url = &rpc[i];

            match self.client.get_job_template(Rpc::new(url.parse().expect("valid rpc url"))).wait() {
                Ok(job_template) => {
                    set.insert(key.clone().to_string(), JobTemplate::from_job(url.clone(),job_template.clone()));
                    //self.shard_job_cache.lock().insert(key.to_string(),job_template);
                }
                Err(ref err) => {
                    let is_method_not_found = if let RpcError::Fail(RpcFail { code, .. }) = err {
                        *code == RpcFailCode::MethodNotFound
                    } else {
                        false
                    };
                    if is_method_not_found {
                        error!(
                            "RPC Method Not Found: \
                         please do checks as follow: \
                         1. if the  server has enabled the Miner API module; \
                         2. If the RPC URL for yee miner is right.",
                        );
                    } else {
                        error!("rpc call get_job_template error: {:?}--shard num={}", err,key);
                    }
                }
            }

        }




        let mut f = false; //更新标记，只要有一个分片数据更新即为true

        if !set.is_empty(){
            for (key, value) in set {
               // debug!("set data---[{}] = {:?}", key, value);

                if self.current_job_set.get(&key).unwrap().clone().rawHash != value.rawHash{
                    f = true;
                }

                self.current_job_set.insert(key.clone(),value.clone());//最终数据全覆盖

              //  self.current_job_set.get_key_value("");
            }


        }else {
            warn!("warning:No data of shard  updates");

        }


        if f {
            let mut work_map:HashMap<String,Work> =  HashMap::new();
            //let len = self.current_job_set.len();
            let  extra_data =  "YeeRoot".as_bytes().to_vec();

            let mut va = vec![];

            let mut sort:HashMap<String,usize> =  HashMap::new();

            let mut i = 0;
            let borrowed_string ="0x".to_string();
            let mut a = CryptoYeeAlgorithm::new();

            for (key, value) in  self.current_job_set.clone() {

                let together = format!("{}{}", borrowed_string, HexDisplay::from(H256::as_fixed_bytes(&value.rawHash.clone())).to_string());
                together.clone().hash(&mut a);
                let h2 = a.hash();
               // println!("h2{}-{:?}",value.rawHash.clone(), h2);
                a.reset();

                va.push(together.clone());
                sort.insert(together.clone(),i);
                i = i+1;
            }

            let mt: MerkleTree<CryptoSHA256Hash, CryptoYeeAlgorithm> =
                MerkleTree::from_iter(va.iter().map(|x|{
                    a.reset();
                    x.hash(&mut a);
                    a.hash()
                }));

            let root = mt.root();
            let leas = mt.clone().leafs;
            //debug!("leas-{:?}", leas);
            //debug!("data-{:?}", data);


            let  merkle_root = root.into();



            for (key, value) in  self.current_job_set.clone() {
                let proof = mt.gen_proof(*sort.get(&format!("{}{}", borrowed_string, HexDisplay::from(H256::as_fixed_bytes(&value.rawHash.clone())).to_string())).unwrap());
                let w = Work{
                    rawHash: value.rawHash,
                    difficulty: value.difficulty,
                    extra_data: extra_data.clone(),
                    merkle_root: merkle_root,
                    merkle_proof: proof.clone(),
                    shard_num: key.parse().unwrap(),
                    shard_cnt: self.map.shards.len() as u32,
                    url:value.url
                };
                // debug!("work---check-{:?}",w);
                // debug!("shard-{}-update! check-{:?}",w.shard_num.clone(),w.clone());


                work_map.insert(key,w);

            }


            let pmap = WorkMap{ work_id: Uuid::new_v4().to_string(),merkle_root,extra_data,work_map };
            if let Err(e) = self.notify_new_work(pmap) {
                error!("gateWay notify_new_work error: {:?}", e);
            }

        }
    }

    fn notify_new_work(&self, work_map: WorkMap) -> Result<(), Error> {
        //debug!("notify_new_work-{:?}",work_map.work_id);
        self.new_work_tx.send(work_map)?;
        Ok(())
    }


}

