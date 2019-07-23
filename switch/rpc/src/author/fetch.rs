
pub use client;
pub use client::ExecutionStrategies;
pub use client::blockchain;
pub use client::backend;
use client::LocalCallExecutor;
pub use substrate_executor ::NativeExecutor;
use std::sync::Arc;
use primitives::Blake2Hasher;
use yee_runtime::opaque::Block;

/// Native executor used for tests.
pub use local_executor::LocalExecutor;

/// Test client database backend.
pub type Backend = client_db::Backend<Block>;




/// Test client executor.
pub type Executor = client::LocalCallExecutor<
    Backend,
    substrate_executor::NativeExecutor<LocalExecutor>,
>;



/// Creates new client instance used for tests.
pub fn new() -> client::Client<Backend, Executor, Block, runtime::RuntimeApi> {
    new_with_backend(Arc::new(Backend::new_test(::std::u32::MAX, ::std::u64::MAX)), false)
}

pub fn new_with_backend<B>(
    backend: Arc<B>,
    support_changes_trie: bool
) -> client::Client<
    B,
    client::LocalCallExecutor<B, executor::NativeExecutor<LocalExecutor>>,
    runtime::Block,
    runtime::RuntimeApi
> where B: backend::LocalBackend<Block, Blake2Hasher>
{
    let executor = NativeExecutor::new(None);
    client::new_with_backend(backend, executor, genesis_storage(support_changes_trie)).unwrap()
}


