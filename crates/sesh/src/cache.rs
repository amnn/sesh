// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Background preview generation and caching.

use std::sync::Arc;

use dashmap::DashMap;
use nucleo::Utf32String;
use tokio_util::task::AbortOnDropHandle;

use crate::picker::Item;

/// Item extension for rendering asynchronously cached previews.
pub(crate) trait Preview: Item {
    /// Render a preview for this item.
    fn preview(&self) -> anyhow::Result<String>;
}

/// Shared preview cache populated by a background worker.
pub(crate) struct PreviewCache<I> {
    entries: Arc<DashMap<Utf32String, Arc<anyhow::Result<String>>>>,
    _workers: Vec<AbortOnDropHandle<()>>,
    _phantom: std::marker::PhantomData<fn(I)>,
}

impl<I: Preview + Send + 'static> PreviewCache<I> {
    /// Start populating previews for the provided sessions in the background.
    pub(crate) fn new(items: Vec<I>) -> Self {
        let entries = Arc::new(DashMap::new());

        let workers = items
            .into_iter()
            .map(|item| {
                let entries = entries.clone();
                let worker = tokio::task::spawn_blocking(move || {
                    let key = Utf32String::from(item.text());
                    let preview = Arc::new(item.preview());
                    entries.insert(key, preview);
                });

                AbortOnDropHandle::new(worker)
            })
            .collect();

        Self {
            entries,
            _workers: workers,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Return the cached preview for `key`, if it has finished rendering.
    pub(crate) fn get(&self, key: &Utf32String) -> Option<Arc<anyhow::Result<String>>> {
        self.entries.get(key).as_deref().cloned()
    }
}
