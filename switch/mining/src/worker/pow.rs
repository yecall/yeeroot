use super::{Worker, WorkerMessage};
use crossbeam_channel::{Receiver, Sender};
use rand::{
    distributions::{self as dist, Distribution as _},
    thread_rng,
};
use serde_derive::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;
use crate::job_template::{ProofMulti, JobTemplate, Hash,Task};
use log::{info, error, warn, debug};
use super::Seal;
use primitives::blake2_256;
use parity_codec::{Encode, Decode, Input};
use chrono::prelude::*;
extern crate chrono;

pub struct Dummy {
    start: bool,
    task: Option<Task>,
    seal_tx: Sender<(String, Seal)>,
    worker_rx: Receiver<WorkerMessage>,
}


#[derive(Clone, PartialEq, Eq, Encode, Decode)]
pub struct Data {
    pub extra_data: Vec<u8>,
    /// merkle root of multi-mining headers
    pub merkle_root: Hash,
    /// POW block nonce
    pub nonce: u64,

}

impl Dummy {
    pub fn new(
        seal_tx: Sender<(String, Seal)>,
        worker_rx: Receiver<WorkerMessage>,
    ) -> Self {
        Self {
            start: true,
            task: None,
            seal_tx,
            worker_rx,
        }
    }

    fn poll_worker_message(&mut self) {
        if let Ok(msg) = self.worker_rx.recv() {
            match msg {
                WorkerMessage::NewWork(task) => self.task = Some(task),
                WorkerMessage::Stop => {
                    self.start = false;
                }
                WorkerMessage::Start => {
                    self.start = true;
                }
                WorkerMessage::Run => {

                }

            }
        }
    }

    fn solve(&self, task: &Task, nonce: u64) {

        let data = Data{
            extra_data: task.extra_data.clone(),
            merkle_root: task.merkle_root.clone(),
            nonce
        };

        let hash:Hash = blake2_256( &data.encode()).into();

        let seal = Seal {post_hash:hash, nonce };
       // println!("solve  input-merkleroot-{}-nonce-{}",data.merkle_root,data.nonce);
        //println!("solve hash --{}", seal.post_hash);
        
        if let Err(err) = self.seal_tx.send((task.work_id.clone(), seal)) {
            error!("seal_tx send error {:?}", err);
        }
    }
}

impl Worker for Dummy {
    fn run<G: FnMut() -> u64>(&mut self, mut rng: G) {
        println!("thsi is worker thread id {:?}",thread::current().id());

        loop {
            self.poll_worker_message();
         //  println!("{}-poll_worker_message--{:?}", Local::now().timestamp_millis(),self.start);

            if self.start {
                if let Some(task) = self.task.clone() {
                    self.solve(&task, rng());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use primitives::{blake2_256,Blake2Hasher, H256};

    #[test]
    fn compare() {

        let rawbytes = blake2_256("dw".as_bytes());

        let hash:H256 = rawbytes.into();

        let y:[u8; 32] = hash.into();

        assert_eq!(rawbytes, y);
    }


}
