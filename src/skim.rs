use std::borrow::Cow;
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

use crate::tmux;

#[derive(Clone, Default)]
struct Cache {
    entries: Arc<DashMap<String, CacheEntry>>,
}

struct CacheEntry {
    width: usize,
    height: usize,
    kind: CacheKind,
}

struct CachedItem<T> {
    cache: Cache,
    inner: T,
}

enum CacheKind {
    Command(String, Option<PreviewPosition>),
    Text(String, Option<PreviewPosition>),
    AnsiText(String, Option<PreviewPosition>),
    Global,
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
    fn text(&self) -> Cow<'_, str> {
        self.inner.text()
    }

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
}

pub fn run(sessions: Vec<tmux::Session>) {
    let options = SkimOptionsBuilder::default()
        .height("90%".to_owned())
        .reverse(true)
        .preview(Some("".to_owned()))
        .preview_window(PreviewLayout::from("down:60%"))
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
