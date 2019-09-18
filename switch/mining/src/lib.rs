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

pub mod job;
pub mod client;
pub mod config;
pub mod job_template;
use crate::job_template::{ProofMulti,JobTemplate,Hash,DifficultyType};
pub mod worker;
pub mod miner;
pub mod gateway;
use std::collections::HashMap;
use std::thread;
use crossbeam_channel::unbounded;
use crate::client::Client;
use log::{info,error,warn,debug};
use crate::miner::Miner;
use crate::config::{WorkerConfig,NodeConfig,MinerConfig,ClientConfig};
use crate::gateway::Gateway;
use yee_switch_rpc::Config;
use yee_consensus_pow::pow::OriginalMerkleProof;
use runtime_primitives::traits::{Hash as HashT, BlakeTwo256};

#[derive(  Debug)]
pub struct Work {
    pub rawHash:Hash,
    pub difficulty: DifficultyType,
    /// Extra Data used to encode miner info AND more entropy
    pub extra_data: Vec<u8>,
    /// merkle root of multi-mining headers
    pub merkle_root: Hash,
    /// merkle tree spv proof
    pub merkle_proof: Vec<Hash>,
    pub url:String,
    pub original_proof: OriginalMerkleProof<BlakeTwo256>,
}
#[derive( Debug)]
pub struct WorkMap {
    pub work_id: String,
    pub merkle_root: Hash,
    pub extra_data: Vec<u8>,
    pub work_map: HashMap<String,Work>,

}

pub fn run(c: Config,interval:u64) {

    let config = ClientConfig {
        poll_interval: interval,
        job_on_submit: true
    };

    let (new_work_tx, new_work_rx) = unbounded();

    let workerc = WorkerConfig{ threads: 1 };

    let  client = Client::new( config.clone());

    let mut gateway = Gateway::new(client.clone(),new_work_tx,c);

    let mut miner =  Miner::new(client.clone(),new_work_rx,workerc.clone());

    thread::Builder::new()
        .name("gateway".to_string())
        .spawn(move || gateway.poll_job_template())
        .expect("Start gateway failed!");

    miner.run();

}