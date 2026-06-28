// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Rendering and state for the session preview pane.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Text;

use crate::app::component::scroll;
use crate::app::component::scroll::Scroll;
use crate::model::prefetch::Prefetch;
use crate::model::session::Session;

/// View over the currently selected session's cached preview content.
pub(crate) struct Preview<'s> {
    selected: Option<&'s Session>,
}

/// Mutable preview pane state shared across renders.
pub(crate) struct State {
    cache: Prefetch<Session>,
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
        let text = Scroll::new(self.text(state));
        f.render_stateful_widget(&text, area, &mut state.scroll);
    }

    /// Resolve the selected session's cached preview text.
    fn text(&self, state: &State) -> Text<'static> {
        let Some(session) = self.selected else {
            return Text::raw("");
        };

        let Some(preview) = state.cache.get(session) else {
            return Text::raw("Loading...");
        };

        match preview.as_ref() {
            Ok(preview) => preview.clone(),
            Err(err) => Text::raw(format!("Error: {err}")),
        }
    }
}

impl State {
    /// Create preview pane state with an empty cache.
    pub(crate) fn new() -> Self {
        Self {
            cache: Prefetch::new(),
            scroll: scroll::State::default(),
            visible: true,
        }
    }

    /// Start generating previews for sessions that are not already cached.
    pub(crate) fn feed<'a>(&mut self, sessions: impl IntoIterator<Item = &'a Session>) {
        self.cache.feed(sessions);
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
