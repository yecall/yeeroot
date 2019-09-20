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

use network::{IdentifySpecialization, PeerId, IdentifyInfo};
use regex::Regex;
use log::debug;

const USERAGENT_SHARD: &str = "shard";

pub struct ShardingIdentifySpecialization {
    protocol_version: String,
    shard_num: u16,
    shard_re: Regex,
}

impl ShardingIdentifySpecialization {
    pub fn new(protocol_version: String, shard_num: u16) -> Self {
        let shard_re = Regex::new(&format!(r"{}/(\d+)", USERAGENT_SHARD)).unwrap();

        ShardingIdentifySpecialization {
            protocol_version,
            shard_num,
            shard_re,
        }
    }

    fn resolve_shard_num(&self, user_agent: &str) -> Option<u16> {
        let m: Vec<&str> = self.shard_re.captures_iter(&user_agent).map(|c| c.get(1).map(|x|x.as_str()).unwrap_or("")).collect();
        m.get(0).and_then(|x| x.parse().ok())
    }

    fn user_agent_match(&self, identify_info: Option<&IdentifyInfo>) -> bool{
        match identify_info {
            Some(identify_info) => match self.resolve_shard_num(identify_info.agent_version.as_str()) {
                Some(shard_num) => self.shard_num == shard_num,
                None => {
                    debug!(target: "sharding", "User agent not match, shard_num: {}, identify_info: {:?}", self.shard_num, identify_info);
                    false
                }
            },
            None => {
                debug!(target: "sharding", "User agent not match, shard_num: {}, identify_info: {:?}", self.shard_num, identify_info);
                false
            }
        }
    }
}

impl IdentifySpecialization for ShardingIdentifySpecialization {
    fn customize_protocol_version(&self, _protocol_version: &str) -> String {
        let protocol_version = self.protocol_version.clone();
        protocol_version
    }

    fn customize_user_agent(&self, user_agent: &str) -> String {
        let user_agent = format!("{} {}/{}", user_agent, USERAGENT_SHARD, self.shard_num);
        user_agent
    }

    fn should_add_discovered_node(&self, _peer_id: &PeerId, identify_info: Option<&IdentifyInfo>) -> bool {

        self.user_agent_match(identify_info)
    }

    fn should_accept_identify_info(&self, _peer_id: &PeerId, identify_info: &IdentifyInfo) -> bool {
        if !identify_info.protocol_version.contains(&self.protocol_version) {
            debug!(target: "sharding", "Protocol version not match, identify_info: {:?}", identify_info);
            return false;
        }
        self.user_agent_match(Some(identify_info))
    }
}