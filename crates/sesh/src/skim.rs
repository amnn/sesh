// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Skim-based session selector UI.

use std::borrow::Cow;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use skim::PreviewPosition;
use skim::prelude::ItemPreview;
use skim::prelude::PreviewContext;
use skim::prelude::Skim;
use skim::prelude::SkimItem;
use skim::prelude::SkimOptionsBuilder;
use skim::prelude::unbounded;
use skim::tui::options::PreviewLayout;

use crate::path::TruncatedExt as _;
use crate::session::Session;

/// Shared UI state for the skim session picker.
#[derive(Clone, Debug, Default)]
pub struct State {
    current_repo: Option<PathBuf>,
}

/// Shared cache for skim preview content.
#[derive(Clone, Default)]
struct Cache {
    entries: Arc<DashMap<String, CacheEntry>>,
}

/// Cached preview content for one skim item and pane size.
struct CacheEntry {
    width: usize,
    height: usize,
    kind: CacheKind,
}

/// Cached representation of `ItemPreview` variants.
enum CacheKind {
    Command(String, Option<PreviewPosition>),
    Text(String, Option<PreviewPosition>),
    AnsiText(String, Option<PreviewPosition>),
    Global,
}

/// Wrapper that adds preview caching to any `SkimItem`.
struct CachedItem<T> {
    cache: Cache,
    inner: T,
}

impl State {
    /// Construct state for a picker launched from `current_repo`.
    pub fn new(current_repo: Option<PathBuf>) -> Self {
        Self { current_repo }
    }

    /// Return the repository containing the current working directory, if any.
    pub fn current_repo(&self) -> Option<&Path> {
        self.current_repo.as_deref()
    }

    fn header(&self) -> String {
        match self.current_repo() {
            Some(repo) => format!("Current repo: {}", repo.truncated()),
            None => "Current repo: none".to_owned(),
        }
    }
}

impl<T: SkimItem> CachedItem<T> {
    fn new(cache: Cache, inner: T) -> Self {
        Self { cache, inner }
    }
}

impl CacheKind {
    fn cache(preview: &ItemPreview) -> Self {
        match preview {
            ItemPreview::Command(cmd) => CacheKind::Command(cmd.to_owned(), None),
            ItemPreview::CommandWithPos(cmd, pos) => CacheKind::Command(cmd.to_owned(), Some(*pos)),
            ItemPreview::Text(text) => CacheKind::Text(text.to_owned(), None),
            ItemPreview::TextWithPos(text, pos) => CacheKind::Text(text.to_owned(), Some(*pos)),
            ItemPreview::AnsiText(text) => CacheKind::AnsiText(text.to_owned(), None),
            ItemPreview::AnsiWithPos(text, pos) => CacheKind::AnsiText(text.to_owned(), Some(*pos)),
            ItemPreview::Global => CacheKind::Global,
        }
    }

    fn preview(&self) -> ItemPreview {
        match self {
            CacheKind::Command(cmd, None) => ItemPreview::Command(cmd.to_owned()),
            CacheKind::Command(cmd, Some(pos)) => ItemPreview::CommandWithPos(cmd.to_owned(), *pos),
            CacheKind::Text(text, None) => ItemPreview::Text(text.to_owned()),
            CacheKind::Text(text, Some(pos)) => ItemPreview::TextWithPos(text.to_owned(), *pos),
            CacheKind::AnsiText(text, None) => ItemPreview::AnsiText(text.to_owned()),
            CacheKind::AnsiText(text, Some(pos)) => ItemPreview::AnsiWithPos(text.to_owned(), *pos),
            CacheKind::Global => ItemPreview::Global,
        }
    }
}

impl<T: SkimItem> SkimItem for CachedItem<T> {
    fn preview(&self, context: PreviewContext) -> ItemPreview {
        let width = context.width;
        let height = context.height;

        if let Some(c) = self.cache.entries.get(self.inner.text().as_ref())
            && c.width == width
            && c.height == height
        {
            return c.kind.preview();
        }

        let preview = self.inner.preview(context);
        self.cache.entries.insert(
            self.inner.text().into_owned(),
            CacheEntry {
                width,
                height,
                kind: CacheKind::cache(&preview),
            },
        );

        preview
    }

    fn text(&self) -> Cow<'_, str> {
        self.inner.text()
    }
}

/// Run the interactive skim picker for discovered sessions.
pub fn run(sessions: Vec<Session>, state: State) {
    let options = SkimOptionsBuilder::default()
        .reverse(true)
        .header(Some(state.header()))
        .preview(Some("".to_owned()))
        .preview_window(PreviewLayout::from("right:60%"))
        .prompt("Session: ".to_owned())
        .build()
        .unwrap();

    let cache = Cache::default();

    let (tx, rx) = unbounded();
    for session in sessions {
        let item = CachedItem::new(cache.clone(), session);
        tx.send(Arc::new(item) as Arc<dyn SkimItem>).ok();
    }

    drop(tx);
    let _ = Skim::run_with(options, Some(rx));
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::State;

    #[test]
    fn renders_header_with_current_repo() {
        let state = State::new(Some(PathBuf::from("/tmp/repo")));

        assert_eq!(state.header(), "Current repo: /tmp/repo");
    }

    #[test]
    fn renders_header_without_current_repo() {
        let state = State::default();

        assert_eq!(state.header(), "Current repo: none");
    }
}
