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

use parity_codec::{Decode, Encode};
use runtime_primitives::generic::DigestItem;
use crate::ScheduledChange;

pub const GENERATED_MODULE_LOG_PREFIX: u8 = 3;
pub const GENERATED_CHANGE_PREFIX: u8 = 0;
pub const GENERATED_FORCE_CHANGE_PREFIX: u8 = 1;

pub trait CrfgChangeDigestItem<N>: Sized {
    fn change(change: ScheduledChange<N>) -> Self;
    fn as_change(&self) -> Option<ScheduledChange<N>>;
}

impl<N, Hash, AuthorityId, SealSignature> CrfgChangeDigestItem<N> for DigestItem<Hash, AuthorityId, SealSignature> where
    N: Decode + Encode,
{
    fn change(change: ScheduledChange<N>) -> Self {
        let prefix: [u8; 2] = [GENERATED_MODULE_LOG_PREFIX, GENERATED_CHANGE_PREFIX];
        let data = Encode::encode(&(prefix, change.delay, change.next_authorities));
        DigestItem::Other(data)
    }

    fn as_change(&self) -> Option<ScheduledChange<N>> {
        match self {
            DigestItem::Other(data) if data.len() >= 4
                && data[0] == GENERATED_MODULE_LOG_PREFIX
                && data[1] == GENERATED_CHANGE_PREFIX => {
                let input = &mut &data[2..];
                let delay = Decode::decode(input)?;
                let next_authorities = Decode::decode(input)?;
                Some(ScheduledChange {
                    delay,
                    next_authorities,
                })
            }
            _ => None
        }
    }
}


pub trait CrfgForceChangeDigestItem<N>: Sized {
    fn force_change(median: N, change: ScheduledChange<N>) -> Self;
    fn as_force_change(&self) -> Option<(N, ScheduledChange<N>)>;
}

impl<N, Hash, AuthorityId, SealSignature> CrfgForceChangeDigestItem<N> for DigestItem<Hash, AuthorityId, SealSignature> where
    N: Decode + Encode,
{
    fn force_change(median: N, change: ScheduledChange<N>) -> Self {
        let prefix: [u8; 2] = [GENERATED_MODULE_LOG_PREFIX, GENERATED_FORCE_CHANGE_PREFIX];
        let data = Encode::encode(&(prefix, median, change.delay, change.next_authorities));
        DigestItem::Other(data)
    }

    fn as_force_change(&self) -> Option<(N, ScheduledChange<N>)> {
        match self {
            DigestItem::Other(data) if data.len() >= 4
                && data[0] == GENERATED_MODULE_LOG_PREFIX
                && data[1] == GENERATED_FORCE_CHANGE_PREFIX => {
                let input = &mut &data[2..];
                let median = Decode::decode(input)?;
                let delay = Decode::decode(input)?;
                let next_authorities = Decode::decode(input)?;
                Some((median, ScheduledChange {
                    delay,
                    next_authorities,
                }))
            }
            _ => None
        }
    }
}

