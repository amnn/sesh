// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Scrollable text component.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::ScrollDirection;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget as _;

use crate::app::component::scrollbar;

/// Scrollable renderable text with no wrapping.
pub(crate) struct Scroll {
    text: Text<'static>,
}

/// Scroll position state for [`Scroll`].
pub(crate) type State = ScrollbarState;

impl Scroll {
    /// Create a scrollable text view from `text`.
    pub(crate) fn new(text: Text<'static>) -> Self {
        Self { text }
    }
}

impl StatefulWidget for &Scroll {
    type State = State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = area.intersection(buf.area);
        if area.is_empty() {
            return;
        }

        let overflow = self.text.height().saturating_sub(area.height as usize);
        let content = if overflow == 0 { 0 } else { overflow + 1 };
        let position = state.get_position().min(content.saturating_sub(1));

        *state = state
            .content_length(content)
            .viewport_content_length(area.height as usize)
            .position(position);

        buf.set_style(area, self.text.style);
        for (line, line_area) in self.text.lines.iter().skip(position).zip(area.rows()) {
            line.render(line_area, buf);
        }

        scrollbar::widget().render(area, buf, state);
    }
}

/// Scroll `state` down by one unit.
pub(crate) fn down(state: &mut State) {
    state.scroll(ScrollDirection::Forward);
}

/// Move `state` to the start of its scrollable content.
pub(crate) fn first(state: &mut State) {
    state.first();
}

/// Scroll `state` up by one unit.
pub(crate) fn up(state: &mut State) {
    state.scroll(ScrollDirection::Backward);
}
