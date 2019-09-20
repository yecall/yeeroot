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

use crate::client::{Client, Rpc};
use crate::config::WorkerConfig;
use crate::worker::{start_worker, WorkerController, WorkerMessage};
use crossbeam_channel::{select, unbounded, Receiver};
use std::thread;
use log::{info, error, warn, debug};
use crate::worker::Seal;
use crate::job_template::{ProofMulti, Hash, Task, DifficultyType, JobResult, ResultDigestItem, WorkProof};
use yee_lru_cache::LruCache;
use yee_util::Mutex;
use crate::WorkMap;

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
        let worker_controller = start_worker(worker, seal_tx.clone());
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
                        error!("Work_rx closed");
                        break;
                    },
                },
                recv(self.seal_rx) -> msg => match msg {
                    Ok((work_id, seal)) => self.check_seal(work_id, seal),
                    _ => {
                        error!("Seal_rx closed");
                        break;
                    },
                }
            };
        }
    }

    fn check_seal(&mut self, work_id: String, seal: Seal) {
        if let Some(work) = self.works.lock().get_refresh(&work_id) {
            let work_set = &work.work_map;
            let mut i = 0;
            let len = work_set.len();

            for (_key, value) in work_set {
                let t = self.verify_target(seal.post_hash, value.difficulty, value.extra_data.clone());
                let mut b = true;
                if let Some(_work) = self.state.lock().get_refresh(&value.raw_hash) {
                    b = false;
                    i = i + 1;
                }

                if t && b {
                    info!("Submit raw_hash: {:?}", value.raw_hash.clone());
                    info!("Submit difficulty: {:?}", value.difficulty.clone());
                    info!("Submit url: {:?}", value.url.clone());
                    info!("Submit merkle_proof: {:?}", value.merkle_proof.clone());
                    let submitjob = ProofMulti {
                        extra_data: value.extra_data.clone(),
                        merkle_root: value.merkle_root.clone(),
                        nonce: seal.nonce,
                        merkle_proof: value.merkle_proof.clone(),
                    };

                    self.state.lock().insert(value.raw_hash.clone(), submitjob.clone());

                    let digest = ResultDigestItem { work_proof: WorkProof::Multi(submitjob) };
                    let job = JobResult { hash: value.raw_hash, digest_item: digest };
                    self.client.submit_job(&job, Rpc::new(value.url.parse().expect("valid rpc url")));
                }
            }

            if i >= len {
                self.notify_workers(WorkerMessage::Stop);
            }
        }
    }

    fn notify_workers(&self, message: WorkerMessage) {
        self.worker_controller.send_message(message.clone());
    }

    fn verify_target(&self, hash: Hash, difficulty: DifficultyType, extra_data: Vec<u8>) -> bool {
        let proof_difficulty = DifficultyType::from(hash.as_ref());

        if extra_data.len() > MAX_EXTRA_DATA_LENGTH || proof_difficulty > difficulty {
            return false;
        }
        return true;
    }
}
