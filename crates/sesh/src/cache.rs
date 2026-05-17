// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Background preview generation and caching.

use std::collections::HashMap;
use std::sync::Arc;

use ansi_to_tui::IntoText as _;
use async_trait::async_trait;
use dashmap::DashMap;
use nucleo::Utf32String;
use ratatui::text::Text;
use tokio_util::task::AbortOnDropHandle;

use crate::picker::Item;

/// Item extension for rendering asynchronously cached previews.
#[async_trait]
pub(crate) trait Preview: Item {
    /// Render a preview for this item.
    async fn preview(&self) -> anyhow::Result<String>;
}

/// Shared preview cache populated by a background worker.
pub(crate) struct PreviewCache {
    entries: Arc<DashMap<Utf32String, Arc<anyhow::Result<Text<'static>>>>>,
    workers: HashMap<Utf32String, AbortOnDropHandle<()>>,
}

impl PreviewCache {
    /// Create an empty preview cache.
    pub(crate) fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            workers: HashMap::new(),
        }
    }

    /// Start populating previews for sessions that are not already cached or pending.
    pub(crate) fn feed<'a, I>(&mut self, items: impl IntoIterator<Item = &'a I>)
    where
        I: Preview + Clone + Send + Sync + 'static,
    {
        self.workers.retain(|_, worker| !worker.is_finished());

        for item in items {
            let key = Utf32String::from(item.text());
            if self.entries.contains_key(&key) || self.workers.contains_key(&key) {
                continue;
            }

            let item = item.clone();
            let entries = self.entries.clone();
            let worker_key = key.clone();
            let worker = tokio::task::spawn(async move {
                let preview = item
                    .preview()
                    .await
                    .and_then(|p| Ok(p.into_bytes().into_text()?));
                entries.insert(key, Arc::new(preview));
            });

            self.workers
                .insert(worker_key, AbortOnDropHandle::new(worker));
        }
    }

    /// Return the cached preview for `key`, if it has finished rendering.
    pub(crate) fn get(&self, key: &Utf32String) -> Option<Arc<anyhow::Result<Text<'static>>>> {
        self.entries.get(key).as_deref().cloned()
    }
}
