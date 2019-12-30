use parity_codec::{Encode, Decode};
use jsonrpc_derive::rpc;
use jsonrpc_core::BoxFuture;
use crate::errors;
use jsonrpc_core::futures::future::{self, Future, IntoFuture};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use runtime_primitives::traits::{Block as BlockT};
use std::marker::PhantomData;
use substrate_primitives::H256;

#[derive(Clone, Serialize, Deserialize, Debug)]
struct RelayData(Vec<u8>);

#[rpc]
pub trait OtherApi<Hash> where
    Hash: Encode,
{
    #[rpc(name = "other_get_relay_extrinsic_data")]
    fn get_relay_extrinsic_data(&self, hash: Hash) -> errors::Result<RelayData>;

    #[rpc(name = "other_recommit_relay_extrinsic")]
    fn recommit_relay_extrinsic(&self, data: Vec<u8>) -> errors::Result<()>;
}

pub struct Other<B> where
    B: BlockT,
{
    _b: PhantomData<B>
}

impl<B> Other<B> where
    B: BlockT,
{
    pub fn new() -> Self {
        Self{
            _b: PhantomData
        }
    }
}

impl<B> OtherApi<B::Hash> for Other<B> where
    B: BlockT,
{
    fn get_relay_extrinsic_data(&self, hash: B::Hash) -> errors::Result<RelayData> {
        // todo
        Ok(RelayData(vec![1u8]))

    }

    fn recommit_relay_extrinsic(&self, data: Vec<u8>) -> errors::Result<()> {
        // todo
        Ok(())
    }
}