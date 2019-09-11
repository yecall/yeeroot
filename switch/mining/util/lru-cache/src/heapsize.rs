extern crate heapsize;

use self::heapsize::HeapSizeOf;
use std::hash::Hash;

use LruCache;

impl<K: Eq + Hash + HeapSizeOf, V: HeapSizeOf> HeapSizeOf for LruCache<K, V> {
    fn heap_size_of_children(&self) -> usize {
        self.inner.heap_size_of_children()
    }
}