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

pub mod pow;
use crate::config::WorkerConfig;
use crossbeam_channel::{unbounded, Sender};
use rand::{random, Rng};
use std::ops::Range;
use std::sync::Arc;
use std::thread;
use crate::job_template::{ProofMulti,JobTemplate,Hash,Task};
use pow::Dummy;
use yee_merkle::proof::Proof;
use log::{info,error,warn,debug};

#[derive(Clone)]
pub enum WorkerMessage {
    Stop,
    Start,
    NewWork(Task),
    Run,
}

pub struct Seal {
    /// hash{extra+root+nonce}
    pub post_hash: Hash,
    /// POW block nonce
    pub nonce: u64,

}

pub struct MerkleSeal {
    pub merkle_root: Hash,
    pub merkle_proof: Proof<[u8;32]>,
}

pub struct WorkerController {
    inner: Vec<Sender<WorkerMessage>>,
}

impl WorkerController {
    pub fn new(inner: Vec<Sender<WorkerMessage>>) -> Self {
        Self { inner }
    }

    pub fn send_message(&self, message: WorkerMessage) {
        for worker_tx in self.inner.iter() {
            if let Err(err) = worker_tx.send(message.clone()) {
                error!("worker_tx send error {:?}", err);
            };
        }
    }
}

fn partition_nonce(id: u64, total: u64) -> Range<u64> {
    let span = u64::max_value() / total;
    let start = span * id;
    let end = match id {
        x if x < total - 1 => start + span,
        x if x == total - 1 => u64::max_value(),
        _ => unreachable!(),
    };
    Range { start, end }
}

fn nonce_generator(range: Range<u64>) -> impl FnMut() -> u64 {
    let mut rng = rand::thread_rng();
    let Range { start, end } = range;
    move || rng.gen_range(start, end)
}


pub fn start_worker(
    config: WorkerConfig,
    seal_tx: Sender<(String, Seal)>,
) -> WorkerController {
                let worker_txs = (0..config.threads)
                    .map(|i| {
                        let worker_name = format!("yee-Worker-{}", i);
                        let nonce_range = partition_nonce(i as u64, config.threads as u64);

                        let (worker_tx, worker_rx) = unbounded();
                        let mut worker = Dummy::new(seal_tx.clone(), worker_rx);

                        thread::Builder::new()
                            .name(worker_name)
                            .spawn(move || {
                                let rng = nonce_generator(nonce_range);
                                worker.run(rng);
                            })
                            .expect("Start worker thread failed");
                        worker_tx
                    })
                    .collect();

                WorkerController::new(worker_txs)
            }


pub trait Worker {
    fn run<G: FnMut() -> u64>(&mut self, rng: G);
}
