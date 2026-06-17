// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Generic one-line list widget with sticky selection highlighting.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::style::Stylize as _;
use ratatui::text::Span;
use ratatui::widgets::ListState;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr as _;

const SELECTED: &str = "▌";

/// A generic one-line list for custom row widgets.
///
/// Works like `ratatui::widgets::List`, but the rows can be custom widgets, they must be of height
/// one, and the styling of selected rows is hardcoded.
pub(crate) struct List<I> {
    items: Vec<I>,
}

impl<I> List<I> {
    /// Create a list from custom items.
    pub(crate) fn new(items: Vec<I>) -> Self {
        Self { items }
    }
}

impl<I> StatefulWidget for List<I>
where
    I: Widget,
{
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        use ratatui::layout::Constraint as C;
        use ratatui::layout::Direction as D;
        use ratatui::layout::Layout as L;

        let area = area.intersection(buf.area);
        if area.is_empty() || self.items.is_empty() {
            return;
        }

        let selected = state.selected();
        let offset = state.offset_mut();

        // If there is a selected row, then clamp the offset so that the selected row is visible.
        if let Some(selected) = selected {
            let lo = selected.saturating_sub(area.height as usize - 1);
            let hi = selected;

            *offset = (*offset).clamp(lo, hi);
        }

        // Then make sure the offset always ensures there's at least one element on screen.
        *offset = (*offset).min(self.items.len() - 1);

        let layout = L::new(
            D::Horizontal,
            vec![C::Length(SELECTED.width() as u16), C::Min(0)],
        );

        for ((i, item), rect) in self
            .items
            .into_iter()
            .enumerate()
            .skip(state.offset())
            .zip(area.rows())
        {
            let selected = state.selected() == Some(i);
            if selected {
                buf.set_style(rect, Style::new().reversed());
            }

            let [margin, rest] = rect.layout(&layout);

            let symbol = if selected {
                Span::raw(SELECTED).on_red()
            } else {
                Span::raw(" ".repeat(SELECTED.width()))
            };

            symbol.render(margin, buf);
            item.render(rest, buf);
        }
    }
}
