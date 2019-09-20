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

use super::{Worker, WorkerMessage};
use crossbeam_channel::{Receiver, Sender};
use crate::job_template::{Hash, Task};
use log::{info, error, warn, debug};
use super::Seal;
use primitives::blake2_256;
use parity_codec::Encode;

extern crate chrono;

pub const MAX_EXTRA_DATA_LENGTH: usize = 32;

pub struct PowSolve {
    start: bool,
    task: Option<Task>,
    seal_tx: Sender<(String, Seal)>,
    worker_rx: Receiver<WorkerMessage>,
}

impl PowSolve {
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
                WorkerMessage::Run => {}
            }
        }
    }

    fn solve(&self, task: &Task, nonce: u64) {
        let data = (task.merkle_root.clone(), task.extra_data.clone(), nonce);
        let hash: Hash = blake2_256(&data.encode()).into();
        let seal = Seal { post_hash: hash, nonce };
        if let Err(err) = self.seal_tx.send((task.work_id.clone(), seal)) {
            error!("Seal_tx send error {:?}", err);
        }
    }
}

impl Worker for PowSolve {
    fn run<G: FnMut() -> u64>(&mut self, mut rng: G) {
        loop {
            self.poll_worker_message();
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
    use primitives::{blake2_256, Blake2Hasher, H256};

    #[test]
    fn compare() {
        let rawbytes = blake2_256("dw".as_bytes());
        let hash: H256 = rawbytes.into();
        let y: [u8; 32] = hash.into();
        assert_eq!(rawbytes, y);
    }
}
