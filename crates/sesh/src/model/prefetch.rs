// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Generic background prefetching and caching.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use tokio_util::task::AbortOnDropHandle;

/// A fetchable cache key.
#[async_trait]
pub(crate) trait Key: Clone + Eq + Hash + Send + Sync + 'static {
    /// Value fetched and cached for this key.
    type Value: Send + Sync + 'static;

    /// Fetch the value for this key.
    async fn fetch(&self) -> anyhow::Result<Self::Value>;
}

/// Shared cache populated by background workers.
pub(crate) struct Prefetch<K: Key> {
    entries: Arc<DashMap<K, Arc<anyhow::Result<K::Value>>>>,
    workers: HashMap<K, AbortOnDropHandle<()>>,
}

impl<K: Key> Prefetch<K> {
    /// Create an empty prefetch cache.
    pub(crate) fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            workers: HashMap::new(),
        }
    }

    /// Start populating values for keys that are not already cached or pending.
    pub(crate) fn feed<'a>(&mut self, keys: impl IntoIterator<Item = &'a K>) {
        self.workers.retain(|_, worker| !worker.is_finished());

        for key in keys {
            if self.entries.contains_key(key) || self.workers.contains_key(key) {
                continue;
            }

            let key = key.clone();
            let entries = self.entries.clone();
            let worker_key = key.clone();
            let worker = tokio::task::spawn(async move {
                let value = key.fetch().await;
                entries.insert(key, Arc::new(value));
            });

            self.workers
                .insert(worker_key, AbortOnDropHandle::new(worker));
        }
    }

    /// Return the cached value for `key`, if it has finished fetching.
    pub(crate) fn get(&self, key: &K) -> Option<Arc<anyhow::Result<K::Value>>> {
        self.entries.get(key).as_deref().cloned()
    }
}
