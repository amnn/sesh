// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Skim-based session selector UI.

use std::borrow::Cow;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use dashmap::DashMap;
use interprocess::bound_util::RefWrite as _;
use interprocess::local_socket::ToNsName as _;
use interprocess::local_socket::traits::Stream as _;
use skim::PreviewPosition;
use skim::fuzzy_matcher::FuzzyMatcher as _;
use skim::fuzzy_matcher::skim::SkimMatcherV2;
use skim::item::MatchedItem;
use skim::item::RankBuilder;
use skim::prelude::ItemPreview;
use skim::prelude::PreviewContext;
use skim::prelude::Skim;
use skim::prelude::SkimItem;
use skim::prelude::SkimOptionsBuilder;
use skim::prelude::SkimOutput;
use skim::prelude::unbounded;
use skim::tui::event::Action;
use skim::tui::event::Event;
use skim::tui::options::PreviewLayout;

use crate::path::TruncatedExt as _;
use crate::session::Session;

const SET_REPO_ACTION: &str = "set-repo";
static NEXT_SOCKET_ID: AtomicUsize = AtomicUsize::new(0);

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

    /// Build the header text shown above the picker.
    fn header(&self) -> String {
        match self.current_repo.as_deref() {
            Some(repo) => format!("Current repo: {}", repo.truncated()),
            None => "Current repo: none".to_owned(),
        }
    }
}

impl<T: SkimItem> CachedItem<T> {
    /// Wrap a skim item with preview caching.
    fn new(cache: Cache, inner: T) -> Self {
        Self { cache, inner }
    }

    /// Return the wrapped item.
    fn inner(&self) -> &T {
        &self.inner
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
pub fn run(sessions: Vec<Session>, mut state: State) {
    let cache = Cache::default();

    let mut query = None;
    let mut selected_row_index = None;
    loop {
        let mut options = SkimOptionsBuilder::default()
            .reverse(true)
            .header(Some(state.header()))
            .preview(Some("".to_owned()))
            .preview_window(PreviewLayout::from("right:60%"))
            .prompt("Session: ".to_owned())
            .query(query.clone())
            .build()
            .unwrap();

        options.keymap.insert(
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
            vec![Action::Accept(Some(SET_REPO_ACTION.to_owned()))],
        );

        if let Some(row) = selected_row_index.filter(|row| *row > 0) {
            let socket_name = next_socket_name();
            options.listen = Some(socket_name.clone());
            restore_selected_row(&socket_name, row);
        }

        let (tx, rx) = unbounded();
        for session in &sessions {
            let item = CachedItem::new(cache.clone(), session.clone());
            tx.send(Arc::new(item) as Arc<dyn SkimItem>).ok();
        }

        drop(tx);
        let Ok(output) = Skim::run_with(options, Some(rx)) else {
            break;
        };

        query = Some(output.query.clone());
        selected_row_index = selected_row(&sessions, &output.query, output.selected_items.first());
        if !handle_output(&mut state, &output) {
            break;
        }
    }
}

/// Apply the accepted skim action and return whether the picker should continue.
fn handle_output(state: &mut State, output: &SkimOutput) -> bool {
    match &output.final_event {
        Event::Action(Action::Accept(Some(action))) if action == SET_REPO_ACTION => {
            if let Some(repo) = selected_repo(output.selected_items.first()) {
                state.current_repo = Some(repo_context_path(repo));
            }
            true
        }
        _ => false,
    }
}

/// Return a unique skim listen socket name for one picker rerun.
fn next_socket_name() -> String {
    let id = NEXT_SOCKET_ID.fetch_add(1, Ordering::Relaxed);
    format!("sesh-{}-{id}", std::process::id())
}

/// Normalize a selected repo path before storing it in the UI state.
fn repo_context_path(repo: &Path) -> PathBuf {
    repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf())
}

/// Re-apply the previous cursor row after reopening skim.
fn restore_selected_row(socket_name: &str, row: usize) {
    let socket_name = socket_name.to_owned();
    thread::spawn(move || {
        let Ok(row) = u16::try_from(row) else {
            return;
        };

        for _ in 0..50 {
            let Ok(ns_name) = socket_name
                .as_str()
                .to_ns_name::<interprocess::local_socket::GenericNamespaced>()
            else {
                return;
            };
            let Ok(stream) = interprocess::local_socket::Stream::connect(ns_name) else {
                thread::sleep(Duration::from_millis(10));
                continue;
            };
            let Ok(action) = ron::ser::to_string(&Action::Down(row)) else {
                return;
            };

            let _ = stream
                .as_write()
                .write_all(format!("{action}\n").as_bytes());
            return;
        }
    });
}

/// Return the repository for the selected item, if the item represents one.
fn selected_repo(item: Option<&Arc<MatchedItem>>) -> Option<&Path> {
    let item = item?;
    let item = item.item.as_any().downcast_ref::<CachedItem<Session>>()?;
    item.inner().repo()
}

/// Return the filtered row index for the selected item, if any.
fn selected_row(
    sessions: &[Session],
    query: &str,
    item: Option<&Arc<MatchedItem>>,
) -> Option<usize> {
    let item = item?;
    let item = item.item.as_any().downcast_ref::<CachedItem<Session>>()?;
    let selected_index = sessions.iter().position(|session| {
        session.name() == item.inner().name() && session.repo() == item.inner().repo()
    })?;

    if query.is_empty() {
        return Some(selected_index);
    }

    let matcher = SkimMatcherV2::default()
        .element_limit(1024 * 1024 * 1024)
        .smart_case();
    let rank_builder = RankBuilder::default();
    let mut rows = sessions
        .iter()
        .enumerate()
        .filter_map(|(index, session)| {
            let text = session.text();
            let (score, matched) = matcher.fuzzy_indices(text.as_ref(), query)?;
            let begin = *matched.first().unwrap_or(&0);
            let end = *matched.last().unwrap_or(&0);
            Some((
                index,
                rank_builder.build_rank(score as i32, begin, end, text.len(), index),
            ))
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|(_, rank)| *rank);

    rows.iter().position(|(index, _)| *index == selected_index)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    use skim::item::MatchedItem;
    use tempfile::tempdir;

    use super::*;
    use crate::session::Session;

    #[test]
    fn canonicalizes_repo_context_path() {
        let temp = tempdir().unwrap();
        let repo = temp.path().join("repo");
        fs::create_dir(&repo).unwrap();

        let relative = repo.strip_prefix(temp.path()).unwrap();
        let cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        assert_eq!(repo_context_path(relative), repo.canonicalize().unwrap());

        std::env::set_current_dir(cwd).unwrap();
    }

    #[test]
    fn finds_selected_row_for_repo_backed_session_item() {
        let sessions = vec![
            Session::from_tmux("runner".to_owned(), None),
            Session::from_repo(PathBuf::from("/tmp/alpha")).unwrap(),
            Session::from_repo(PathBuf::from("/tmp/beta")).unwrap(),
        ];
        let item = Arc::new(MatchedItem {
            item: Arc::new(CachedItem::new(
                Cache::default(),
                Session::from_repo(PathBuf::from("/tmp/beta")).unwrap(),
            )),
            rank: [0; 5],
            matched_range: None,
        });

        assert_eq!(selected_row(&sessions, "", Some(&item)), Some(2));
    }

    #[test]
    fn finds_selected_row_for_filtered_query() {
        let sessions = vec![
            Session::from_tmux("runner".to_owned(), None),
            Session::from_repo(PathBuf::from("/tmp/alpha")).unwrap(),
            Session::from_repo(PathBuf::from("/tmp/beta")).unwrap(),
            Session::from_repo(PathBuf::from("/tmp/gamma")).unwrap(),
        ];
        let item = Arc::new(MatchedItem {
            item: Arc::new(CachedItem::new(
                Cache::default(),
                Session::from_repo(PathBuf::from("/tmp/beta")).unwrap(),
            )),
            rank: [0; 5],
            matched_range: None,
        });

        assert_eq!(selected_row(&sessions, "et", Some(&item)), Some(0));
    }

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

    #[test]
    fn returns_none_for_session_item_without_repo() {
        let item = Arc::new(MatchedItem {
            item: Arc::new(CachedItem::new(
                Cache::default(),
                Session::from_tmux("scratch".to_owned(), None),
            )),
            rank: [0; 5],
            matched_range: None,
        });

        assert_eq!(selected_repo(Some(&item)), None);
    }

    #[test]
    fn returns_repo_for_repo_backed_session_item() {
        let repo = PathBuf::from("/tmp/repo");
        let item = Arc::new(MatchedItem {
            item: Arc::new(CachedItem::new(
                Cache::default(),
                Session::from_repo(repo.clone()).unwrap(),
            )),
            rank: [0; 5],
            matched_range: None,
        });

        assert_eq!(selected_repo(Some(&item)), Some(repo.as_path()));
    }
}
