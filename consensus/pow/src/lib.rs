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

//! POW (Proof of Work) consensus in YeeChain

use std::sync::Arc;
use consensus_common::{
    import_queue::{
        BasicQueue,
        SharedBlockImport, SharedJustificationImport,
    },
};
use runtime_primitives::{
    codec::{
        Decode, Encode,
    },
    traits::{
        AuthorityIdFor, Block, Header,
    },
};

pub use digest::CompatibleDigestItem;
pub use pow::WorkProof;

mod digest;
mod pow;
mod verifier;

type AuthorityId<B> = AuthorityIdFor<B>;

/// POW chain import queue
pub type PowImportQueue<B> = BasicQueue<B>;

/// Start import queue for POW consensus
pub fn import_queue<B, C, E>(
    block_import: SharedBlockImport<B>,
    justification_import: Option<SharedJustificationImport<B>>,
    client: Arc<C>,
) -> Result<PowImportQueue<B>, consensus_common::Error> where
    B: Block,
    C: 'static + Send + Sync,
{
    let verifier = Arc::new(
        verifier::PowVerifier {
            client,
        }
    );
    Ok(BasicQueue::new(verifier, block_import, justification_import))
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
