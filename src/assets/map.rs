use slotmap::{secondary, SecondaryMap};

use super::{AssetId, Handle};

/// Associates handles with locally owned data
pub struct HandleMap<K, V> {
    inner: SecondaryMap<AssetId, V>,
    _marker: std::marker::PhantomData<K>,
}

impl<K, V> HandleMap<K, V> {
    pub fn new() -> Self {
        Self {
            inner: SecondaryMap::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn insert(&mut self, handle: Handle<K>, value: V) {
        self.inner.insert(handle.id(), value);
    }

    pub fn get(&self, handle: &Handle<K>) -> Option<&V> {
        self.inner.get(handle.id())
    }

    pub fn get_mut(&mut self, handle: &Handle<K>) -> Option<&mut V> {
        self.inner.get_mut(handle.id())
    }

    pub fn entry(&mut self, handle: Handle<K>) -> Entry<K, V> {
        Entry {
            entry: self.inner.entry(handle.id()).expect("Invalid handle"),
            handle,
        }
    }
}

pub struct Entry<'a, K, V> {
    entry: secondary::Entry<'a, AssetId, V>,
    handle: Handle<K>,
}

impl<'a, K, V> Entry<'a, K, V> {
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        Entry {
            entry: self.entry.and_modify(f),
            handle: self.handle,
        }
    }

    pub fn or_insert(self, default: V) -> &'a mut V {
        self.entry.or_insert(default)
    }

    pub fn or_insert_with<F>(self, default: F) -> &'a mut V
    where
        F: FnOnce() -> V,
    {
        self.entry.or_insert_with(default)
    }

    pub fn or_insert_with_key<F>(self, default: F) -> &'a mut V
    where
        F: FnOnce(Handle<K>) -> V,
    {
        self.entry.or_insert_with(|| default(self.handle))
    }
}

impl<K, V> Default for HandleMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
