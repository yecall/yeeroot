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

use crate::client::{Client,Rpc};
use crate::config::WorkerConfig;
use crate::worker::{start_worker, WorkerController, WorkerMessage};
use crate::Work;
use crossbeam_channel::{select, unbounded, Receiver};
use std::sync::Arc;
use std::thread;
use log::{info,error,warn,debug};
use crate::worker::Seal;
use crate::job_template::{ProofMulti,JobTemplate,Hash,Task,DifficultyType};
use yee_lru_cache::LruCache;
use yee_util::Mutex;
use crate::WorkMap;
use std::collections::HashMap;
use std::convert::TryInto;
use core::borrow::{BorrowMut, Borrow};
use chrono::prelude::*;
extern crate chrono;
use primitives::hexdisplay::HexDisplay;
use crate::job::{JobResult,ResultDigestItem,WorkProof};
use std::io::Read;
use yee_consensus_pow::pow::{MiningHash,MiningAlgorithm,CompactMerkleProof,OriginalMerkleProof};
use runtime_primitives::traits::{Hash as HashT, BlakeTwo256};
use yee_serde_hex::SerdeHex;
//use yee_rpc::mining::primitives::JobResult;

const WORK_CACHE_SIZE: usize = 32;
/// Max length in bytes for pow extra data
pub const MAX_EXTRA_DATA_LENGTH: usize = 32;

pub struct Miner {
    pub client: Client,
    pub worker_controller: WorkerController,
    pub work_rx: Receiver<WorkMap>,
    pub seal_rx: Receiver<(String, Seal)>,
    pub works: Mutex<LruCache<String, WorkMap>>,
    pub state: Mutex<LruCache<Hash, ProofMulti>>,

}

impl Miner {
    pub fn new(
        client: Client,
        work_rx: Receiver<WorkMap>,
        worker: WorkerConfig,
    ) -> Miner {
        let (seal_tx, seal_rx) = unbounded();
        let worker_controller = start_worker(worker,seal_tx.clone());

        Miner {
            works: Mutex::new(LruCache::new(WORK_CACHE_SIZE)),
            state: Mutex::new(LruCache::new(WORK_CACHE_SIZE)),
            client,
            worker_controller,
            work_rx,
            seal_rx,
        }
    }

    pub fn run(&mut self) {
        //debug!("thsi is miner run thread id {:?}",thread::current().id());
        loop {
            self.notify_workers(WorkerMessage::Run);
            select! {
                recv(self.work_rx) -> msg => match msg {
                    Ok(work) => {
                       // println!("get new work .......");
                        let work_id = work.work_id.clone();
                        let merkle_root = work.merkle_root.clone();
                        let extra_data = work.extra_data.clone();
                       //  debug!("cache_and send_WorkerMessage: {}", work_id);
                        self.works.lock().insert(work_id.clone(), work);

                        let task = Task{
                                    work_id: work_id,
                                    extra_data: extra_data,
                                    merkle_root: merkle_root
                                   };
                        self.notify_workers(WorkerMessage::Start);
                        self.notify_workers(WorkerMessage::NewWork(task));
                    },
                    _ => {
                        error!("work_rx closed");
                        break;
                    },
                },
                recv(self.seal_rx) -> msg => match msg {
                    Ok((work_id, seal)) => self.check_seal(work_id, seal),
                    _ => {
                        error!("seal_rx closed");
                        break;
                    },
                }
            };
        }
    }

    fn check_seal(&mut self, work_id: String, seal: Seal) {
       // debug!("now  check_seal  work_id:");
        if let Some(work) = self.works.lock().get_refresh(&work_id) {
            //debug!("{}--now  check_seal  work_id: {}",  Local::now().timestamp_millis(),work_id);
            let mut work_set = &work.work_map;
            let mut i = 0;
            let len = work_set.len();

            for (key, value) in  work_set {
                let t =  self.verify_target(seal.post_hash,value.difficulty,value.extra_data.clone());
                let m =  self.verify_merkel_proof(&value.original_proof);
                let m = true;
                let mut b = true;
                if let Some(work) = self.state.lock().get_refresh(&value.rawHash) {
                    b = false;
                    i = i+1;
                }

                if(t&&m&&b){
                    let submitjob = ProofMulti {
                        extra_data: value.extra_data.clone(),
                        merkle_root: value.merkle_root.clone(),
                        nonce: seal.nonce,
                        merkle_proof: value.merkle_proof.clone()
                    };
                    debug!("find seal-{}:{} ,now  submit_job  work_id: {:?}", Local::now().time(),value.rawHash.clone(), submitjob);
                    //  debug!("find seal-{}:{} ", Local::now().time(),value.rawHash.clone());
                    // debug!("                                 ");
                    // debug!("--{}",value.url.clone());
                    //  debug!("-format-{:?}", value.extra_data.clone());
                    self.state.lock().insert(value.rawHash.clone(), submitjob.clone());
//                    let p = crate::job::ProofMulti{
//                        extra_data: "0x010203040506".to_string(),
//                        merkle_root: submitjob.merkle_root,
//                        nonce: "0x400".to_string(),
//                        merkle_proof: "0x150bb6eaccbbe063541a313834a1a9e8ead4c3247a9c164197fed7b15a535386".to_string()
//                    };

                    let digest = ResultDigestItem{ work_proof: WorkProof::Multi(submitjob)};
                    let job = JobResult{ hash: value.rawHash, digest_item: digest };
                    self.client.submit_job(&job,Rpc::new(value.url.parse().expect("valid rpc url")));
                }

            }

            if i >= len{//所有分片都出块了
                //debug!("WorkerMessage::Stop-i-{}",i.clone());
                  self.notify_workers(WorkerMessage::Stop);
            }
        }

    }

    fn notify_workers(&self, message: WorkerMessage) {
            self.worker_controller.send_message(message.clone());
    }

    fn verify_target(&self,hash:Hash,difficulty:DifficultyType, extra_data: Vec<u8>)-> bool{

        let proof_difficulty = DifficultyType::from(hash.as_ref());

        if extra_data.len() > MAX_EXTRA_DATA_LENGTH || proof_difficulty > difficulty{
            return false;
        }
        return true;
    }

    fn verify_merkel_proof(&self,original_proof:&OriginalMerkleProof<BlakeTwo256>)-> bool {

        return   original_proof.proof.validate::<MiningAlgorithm<BlakeTwo256>>();

    }

}
