use{
    runtime_primitives::{
        traits::Block,
    }
};

pub struct RelayClient<C>{
    client: Arc<C>
}

impl<C> RelayClient<C>{
    pub fn new(client:Arc<C>) -> Self{
        RelayClient{
            client
        }
    }
}

fn start_relay_transfer<C>(client:Arc<C>){

}