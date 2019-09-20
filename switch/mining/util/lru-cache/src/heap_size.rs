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

extern crate heapsize;
use self::heapsize::HeapSizeOf;
use std::hash::Hash;
use LruCache;

impl<K: Eq + Hash + HeapSizeOf, V: HeapSizeOf> HeapSizeOf for LruCache<K, V> {
    fn heap_size_of_children(&self) -> usize {
        self.inner.heap_size_of_children()
    }
}