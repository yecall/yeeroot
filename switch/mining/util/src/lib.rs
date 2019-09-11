
pub use parking_lot::{
    self, Condvar, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::time::Duration;

const TRY_LOCK_TIMEOUT: Duration = Duration::from_secs(300);

pub fn lock_or_panic<T>(data: &Mutex<T>) -> MutexGuard<T> {
    data.try_lock_for(TRY_LOCK_TIMEOUT)
        .expect("please check if reach a deadlock")
}

