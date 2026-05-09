// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Background preview generation and caching.

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
pub(crate) struct PreviewCache<I> {
    entries: Arc<DashMap<Utf32String, Arc<anyhow::Result<Text<'static>>>>>,
    _workers: Vec<AbortOnDropHandle<()>>,
    _phantom: std::marker::PhantomData<fn(I)>,
}

impl<I: Preview + Send + Sync + 'static> PreviewCache<I> {
    /// Start populating previews for the provided sessions in the background.
    pub(crate) fn new(items: Vec<I>) -> Self {
        let entries = Arc::new(DashMap::new());

        let workers = items
            .into_iter()
            .map(|item| {
                let entries = entries.clone();
                let worker = tokio::task::spawn(async move {
                    let key = Utf32String::from(item.text());
                    let preview = item
                        .preview()
                        .await
                        .and_then(|p| Ok(p.into_bytes().into_text()?));
                    entries.insert(key, Arc::new(preview));
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
    pub(crate) fn get(&self, key: &Utf32String) -> Option<Arc<anyhow::Result<Text<'static>>>> {
        self.entries.get(key).as_deref().cloned()
    }
}
