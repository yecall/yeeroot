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

use yee_foreign_network as network;
use substrate_service::{TaskExecutor, Arc};
use substrate_cli::error;
use log::{info, warn};
use tokio::timer::Interval;
use std::time::{Instant, Duration, SystemTime, UNIX_EPOCH};
use futures::stream::Stream;
use futures::future::Future;
use yee_runtime::opaque::{Block, UncheckedExtrinsic};
use primitives::H256;

pub struct DemoParams {
    pub shard_num: u16,
}

pub fn start_foreign_demo(
    param: DemoParams,
    foreign_network: Box<Arc<network::SyncProvider<Block, H256>>>,
    executor: &TaskExecutor,
)
    -> error::Result<()>
{
    let status = foreign_network.network_state();

    info!("foreign demo: status: {:?}", status);

    let foreign_network_clone = foreign_network.clone();

    let task = Interval::new(Instant::now(), Duration::from_secs(3)).for_each(move |_instant| {

        let extrinsics = gen_extrinsics();

        let target_shard_num = (param.shard_num + 1) % 4;

        info!("foreign demo: sent relay extrinsics: shard_num: {} extrinsics: {:?}", target_shard_num, extrinsics);

        foreign_network_clone.on_relay_extrinsics(target_shard_num, extrinsics);

        Ok(())
    }).map_err(|e| warn!("Foreign demo error: {:?}", e));

    let message_task = foreign_network.out_messages().for_each(move |messages| {

        info!("foreign demo: received messages: {:?}", messages);

        Ok(())
    });

    executor.spawn(task);
    executor.spawn(message_task);

    Ok(())
}

fn gen_extrinsics() -> Vec<(H256, UncheckedExtrinsic)> {

    let mut result = Vec::new();
    for i in simple_rand_array(3) {

        let hash = H256::from(i);
        let extrinsic = UncheckedExtrinsic(i.to_vec());

        result.push((hash, extrinsic))
    }

    result
}

fn simple_rand_array(count: usize) -> Vec<[u8; 32]>{
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("qed").as_millis();

    let mut result  = Vec::new();
    for i in 0..count{
        let tmp = now + i as u128;
        let mut array = [0u8; 32];
        array[0] = (tmp%256) as u8;
        array[1] = (tmp/256%256) as u8;
        array[3] = (tmp/256/256%256) as u8;

        result.push(array);
    }
    result
}
