// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Rendering for the `onto` revision picker.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;
use ratatui::widgets::ScrollbarState;

use crate::app::scrollbar;

/// Onto-picker view over renderable log text for the current repo context.
pub(super) struct OntoPicker {
    text: Text<'static>,
}

/// Mutable state owned by the onto-picker preview surface.
#[derive(Default)]
pub(super) struct State {
    scroll: ScrollbarState,
}

impl OntoPicker {
    /// Create an onto-picker view over renderable `jj log` text.
    pub(super) fn new(text: Text<'static>) -> Self {
        Self { text }
    }

    /// Render the onto picker into `area` using log text as its source.
    pub(super) fn draw(&self, f: &mut Frame<'_>, area: Rect, state: &mut State) {
        let overflow = self.text.height().saturating_sub(area.height as usize);
        let content = if overflow == 0 { 0 } else { overflow + 1 };

        state.scroll = state
            .scroll
            .content_length(content)
            .viewport_content_length(area.height as usize);

        let paragraph =
            Paragraph::new(self.text.clone()).scroll((state.scroll.get_position() as u16, 0));

        f.render_widget(paragraph, area);
        f.render_stateful_widget(scrollbar::widget(), area, &mut state.scroll);
    }
}
