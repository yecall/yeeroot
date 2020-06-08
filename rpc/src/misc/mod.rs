use std::sync::Arc;
use std::hash::Hasher;
use parity_codec::{Encode, Decode};
use jsonrpc_derive::rpc;
use jsonrpc_core::BoxFuture;
use crate::errors;
use jsonrpc_core::futures::future::{self, Future, IntoFuture};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde_json::Value;
use std::marker::PhantomData;
use yee_primitives::RecommitRelay;
use futures::sync::mpsc;

#[rpc]
pub trait MiscApi<Hash> {
    #[rpc(name = "chain_recommitRelay")]
    fn recommit_relay_extrinsic(&self, hash: Hash, index: usize) -> errors::Result<()>;
}

pub struct Misc<Hash> {
    recommit_relay_sender: Arc<Option<mpsc::UnboundedSender<RecommitRelay<Hash>>>>,
}

impl<Hash> Misc<Hash> {
    pub fn new(recommit_relay_sender: Arc<Option<mpsc::UnboundedSender<RecommitRelay<Hash>>>>) -> Self {
        Self {
            recommit_relay_sender
        }
    }
}

impl<Hash> MiscApi<Hash> for Misc<Hash> where
    Hash: Send + Sync + 'static,
{
    fn recommit_relay_extrinsic(&self, hash: Hash, index: usize)  -> errors::Result<()> {
        let recommit_param = RecommitRelay {
            hash,
            index
        };
        let sender = self.recommit_relay_sender.as_ref();
        sender.as_ref().unwrap().unbounded_send(recommit_param);
        Ok(())
    }
}