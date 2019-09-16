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
use yee_merkle::proof::Proof;
use crate::merkle::{CryptoYeeAlgorithm};
use yee_merkle::hash::{Algorithm, Hashable};
use primitives::hexdisplay::HexDisplay;
use crate::job::{JobResult,ResultDigestItem,WorkProof};
use std::io::Read;

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
                        let  extra_data = work.extra_data.clone();

                      //  println!("cache_and send_WorkerMessage: {}", work_id);
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

          //  debug!("{}--now  check_seal  work_id: {}",  Local::now().timestamp_millis(),work_id);

            let mut work_set = &work.work_map;

            let mut i = 0;
            let len = work_set.len();

            for (key, value) in  work_set {

                let t =  self.verify_target(seal.post_hash,value.difficulty,value.extra_data.clone());
                let m =  self.verify_merkel_proof(value.rawHash,value.merkle_proof.clone());
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
                        shard_num: value.shard_num.clone(),
                        shard_cnt: value.shard_cnt.clone(),
                        merkle_proof: value.merkle_proof.clone()
                    };
                  //  debug!("find seal-{}:{} ,now  submit_job  work_id: {:?}", Local::now().time(),value.rawHash.clone(), submitjob);
                  //  debug!("find seal-{}:{} ", Local::now().time(),value.rawHash.clone());
                   // debug!("                                 ");
                   // debug!("--{}",value.url.clone());

                  //  debug!("-format-{:?}", value.extra_data.clone());


                    self.state.lock().insert(value.rawHash.clone(), submitjob.clone());


                    let p = crate::job::ProofMulti{
                        extra_data: "".to_string(),
                        merkle_root: submitjob.merkle_root,
                        nonce: seal.nonce.to_string(),
                        merkle_proof: "0x150bb6eaccbbe063541a313834a1a9e8ead4c3247a9c164197fed7b15a535386".to_string()
                    };

                    let wp = WorkProof::Multi(p);
                    let dig =ResultDigestItem{ work_proof: wp};


                    let sjob = JobResult{ hash: value.rawHash, digest_item: dig };
                    self.client.submit_job(&sjob,Rpc::new(value.url.parse().expect("valid rpc url")));
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

    fn verify_merkel_proof(&self,rawhash:Hash,merkle_proof: Proof<[u8;32]>)-> bool{
        let mut a = CryptoYeeAlgorithm::new();
        let borrowed_string ="0x".to_string();

        let together = format!("{}{}", borrowed_string, HexDisplay::from(Hash::as_fixed_bytes(&rawhash.clone())).to_string());
        together.clone().hash(&mut a);
        let h2 = a.hash();
        //  debug!("rawhash--{}-Sha256(rawhash{:?}",rawhash.clone(), h2);
        a.reset();

        let item =  merkle_proof.item();

        // debug!("verify_merkel_proof---item-{:?}",item);

        let f =merkle_proof.validate::<CryptoYeeAlgorithm>()&&(h2==item);
        //debug!("verify-fff-{}",f);
        f
    }


}
