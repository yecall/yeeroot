use substrate_service::{
    FactoryBlock, LightComponents, ServiceFactory,
};

pub trait ForeignChains<F> : Send + Sync
    where F: ServiceFactory + Send + Sync,
{
    fn get_shard_component(&self, shard: u16) -> Option<&LightComponents<F>>;
}