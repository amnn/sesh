// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Rendering and state for the session preview pane.

use std::collections::HashMap;
use std::path::PathBuf;

use ansi_to_tui::IntoText as _;
use anyhow::Context as _;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::app::component::loader::Loader;
use crate::app::component::scroll;
use crate::app::component::scroll::Scroll;
use crate::cmd::jj;
use crate::model::session::Session;

/// View over the currently selected session's cached preview content.
pub(crate) struct Preview<'s> {
    selected: Option<&'s Session>,
}

/// Mutable preview pane state shared across renders.
pub(crate) struct State {
    entries: HashMap<PathBuf, Loader<Scroll>>,
    scroll: scroll::State,
    visible: bool,
}

impl<'s> Preview<'s> {
    /// Create a preview view over the currently selected session.
    pub(crate) fn new(selected: Option<&'s Session>) -> Self {
        Self { selected }
    }

    /// Render the preview text and its scrollbar into `area`.
    pub(crate) fn draw(&self, f: &mut Frame<'_>, area: Rect, state: &mut State) {
        let Some(repo) = self.selected.and_then(|s| s.preview_repo()) else {
            return;
        };

        let preview = state
            .entries
            .entry(repo.clone())
            .or_insert_with(|| loader(repo));

        f.render_stateful_widget(&*preview, area, &mut state.scroll);
    }
}

impl State {
    /// Create preview pane state with an empty cache.
    pub(crate) fn new() -> Self {
        Self {
            entries: HashMap::new(),
            scroll: scroll::State::default(),
            visible: true,
        }
    }

    /// Start generating previews for sessions that are not already cached.
    pub(crate) fn feed<'a>(&mut self, sessions: impl IntoIterator<Item = &'a Session>) {
        for repo in sessions.into_iter().filter_map(|s| s.preview_repo()) {
            self.entries
                .entry(repo.clone())
                .or_insert_with(|| loader(repo));
        }
    }

    /// Move the scroll position to the start of the content.
    pub(crate) fn first(&mut self) {
        scroll::first(&mut self.scroll);
    }

    /// Scroll down by one unit.
    pub(crate) fn scroll_down(&mut self) {
        scroll::down(&mut self.scroll);
    }

    /// Scroll up by one unit.
    pub(crate) fn scroll_up(&mut self) {
        scroll::up(&mut self.scroll);
    }

    /// Toggle visibility (also resets scroll position to the start).
    pub(crate) fn toggle(&mut self) {
        self.visible = !self.visible;
        scroll::first(&mut self.scroll);
    }

    /// Return whether the preview pane is currently visible.
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }
}

/// Create a preview loader that loads `repo`'s log in the background.
fn loader(repo: PathBuf) -> Loader<Scroll> {
    Loader::new(async move {
        jj::log(&repo)
            .await
            .with_context(|| format!("failed to build preview for repo '{}'", repo.display()))?
            .into_bytes()
            .into_text()
            .context("failed to render jj log output")
            .map(Scroll::new)
    })
}
