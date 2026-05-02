// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Rendering and state for the session preview pane.

use nucleo::Utf32String;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;
use ratatui::widgets::ScrollDirection;
use ratatui::widgets::ScrollbarState;

use crate::app::scrollbar;
use crate::cache::PreviewCache;
use crate::picker::Item as _;
use crate::session::Session;

/// View over the currently selected session's cached preview content.
pub(super) struct Preview<'s> {
    selected: Option<&'s Session>,
}

/// Mutable preview pane state shared across renders.
pub(super) struct State {
    cache: PreviewCache<Session>,
    scroll: ScrollbarState,
    visible: bool,
}

impl<'s> Preview<'s> {
    pub(super) fn new(selected: Option<&'s Session>) -> Self {
        Self { selected }
    }

    pub(super) fn draw(&self, f: &mut Frame<'_>, area: Rect, state: &mut State) {
        let text = self.text(state);

        let overflow = text.height().saturating_sub(area.height as usize);
        let content = if overflow == 0 { 0 } else { overflow + 1 };

        state.scroll = state
            .scroll
            .content_length(content)
            .viewport_content_length(area.height as usize);

        let paragraph = Paragraph::new(text).scroll((state.scroll.get_position() as u16, 0));

        f.render_widget(paragraph, area);
        f.render_stateful_widget(scrollbar::widget(), area, &mut state.scroll);
    }

    fn text(&self, state: &State) -> Text<'static> {
        let Some(session) = self.selected else {
            return Text::raw("");
        };

        let key = Utf32String::from(session.text());
        let Some(preview) = state.cache.get(&key) else {
            return Text::raw("Loading...");
        };

        match preview.as_ref() {
            Ok(preview) => preview.clone(),
            Err(err) => Text::raw(format!("Error: {err}")),
        }
    }
}

impl State {
    pub(super) fn new(items: Vec<Session>) -> Self {
        Self {
            cache: PreviewCache::new(items),
            scroll: ScrollbarState::default(),
            visible: true,
        }
    }

    /// Move the scroll position to the start of the content.
    pub(super) fn first(&mut self) {
        self.scroll.first();
    }

    /// Scroll down by one unit.
    pub(super) fn scroll_down(&mut self) {
        self.scroll.scroll(ScrollDirection::Forward);
    }

    /// Scroll up by one unit.
    pub(super) fn scroll_up(&mut self) {
        self.scroll.scroll(ScrollDirection::Backward);
    }

    /// Toggle visibility (also resets scroll position to the start).
    pub(super) fn toggle(&mut self) {
        self.visible = !self.visible;
        self.scroll.first();
    }

    /// Return whether the preview pane is currently visible.
    pub(super) fn visible(&self) -> bool {
        self.visible
    }
}
