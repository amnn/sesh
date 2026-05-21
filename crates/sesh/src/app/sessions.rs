// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Component and state for rendering the session list.

use nucleo::Config;
use nucleo::Item;
use nucleo::Matcher;
use nucleo::Utf32Str;
use nucleo::pattern::Pattern;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use ratatui::widgets::ScrollbarState;
use unicode_width::UnicodeWidthStr as _;

use crate::app::list::List;
use crate::app::row::Row;
use crate::app::scrollbar;
use crate::picker::Item as _;
use crate::session::Session;

/// Session-list component, backed by fuzzy-matched rows and an optional new session candidate.
pub(super) struct Sessions<'s> {
    new: Option<Session>,
    rest: &'s [Item<'s, Session>],
    pattern: &'s Pattern,
}

/// Persistent selection and scroll state for the session list.
#[derive(Default)]
pub(super) struct State {
    deleting: bool,
    list: ListState,
    selected: Option<Session>,
}

impl<'s> Sessions<'s> {
    /// Create a new `Sessions` component with `new` representing the potential new session, and
    /// `rest` being the other candidates. The `pattern` is what was used to filter down to these
    /// candidates, and is used to highlight the matching parts of candidate text.
    pub(super) fn new(
        new: Option<Session>,
        rest: &'s [Item<'s, Session>],
        pattern: &'s Pattern,
    ) -> Self {
        Self { new, pattern, rest }
    }

    /// Render the session rows and keep the selected session state in sync with the list.
    pub(super) fn draw(&self, f: &mut Frame<'_>, list: Rect, scroll: Rect, state: &mut State) {
        let mut rows = Vec::with_capacity(self.rest.len() + 1);

        state.selected = match (state.list.selected_mut(), &self.new, self.rest) {
            // If the list is completely empty, then clear the selection. After this case, we can
            // assume that there is at least one session between `new` and `rest`.
            (s, None, []) => {
                *s = None;
                None
            }

            // If the first row has been selected, but it corresponds to the empty new selection,
            // then nudge it into the `rest` list.
            (s @ Some(0), None, [fst, ..]) => {
                *s = Some(1);
                Some(fst.data.clone())
            }

            // If there is no selection, and there are no `rest` sessions, set the selection to the
            // `new` session.
            (s @ None, Some(new), []) => {
                *s = Some(0);
                Some(new.clone())
            }

            // Otherwise, if there is no selection, default to the first `rest` session.
            (s @ None, _, [fst, ..]) => {
                *s = Some(1);
                Some(fst.data.clone())
            }

            // In all other cases, make sure the selected item is clamped by the rows on offer.
            (Some(s), new, rest) => {
                *s = rest.len().min(*s);
                if *s == 0 {
                    new.clone()
                } else {
                    rest.get(*s - 1).map(|i| i.data.clone())
                }
            }
        };

        let selected = state.list.selected();
        if let Some(session) = &self.new {
            rows.push(session.render(selected == Some(0), false, &[]))
        } else {
            rows.push(Row::empty())
        }

        for (i, item) in (1..).zip(self.rest) {
            let mut matcher = Matcher::new(Config::DEFAULT);
            let mut indices = Vec::new();
            let text = item.matcher_columns[0].slice(..);

            self.pattern.indices(text, &mut matcher, &mut indices);
            indices.sort_unstable();
            indices.dedup();

            let highlighted = selected == Some(i);
            let row = item.data.render(highlighted, state.deleting, &indices);

            let margin = indices.last().copied().map(|off| right_margin(text, off));
            rows.push(row.with_right_margin(margin));
        }

        let height = list.height as usize;
        let mut scroll_state = ScrollbarState::default()
            .content_length(rows.len().saturating_sub(height) + 1)
            .viewport_content_length(height)
            .position(state.list.offset());

        f.render_stateful_widget(List::new(rows), list, &mut state.list);
        f.render_stateful_widget(scrollbar::widget(), scroll, &mut scroll_state);
    }
}

impl State {
    /// Create empty session-list state.
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Whether the currently selected session can be closed.
    pub(super) fn can_close(&self) -> bool {
        self.selected.as_ref().is_some_and(Session::can_close)
    }

    /// Whether the currently selected session can be deleted.
    pub(super) fn can_delete(&self) -> bool {
        self.selected.as_ref().is_some_and(Session::can_delete)
    }

    /// Whether the selected session is marked for deletion.
    pub(super) fn is_deleting(&self) -> bool {
        self.deleting
    }

    /// The session to preview, if one is currently selected.
    pub(super) fn preview(&self) -> Option<&Session> {
        self.selected.as_ref()
    }

    /// Cancel any pending deletion.
    pub(super) fn reset_delete(&mut self) {
        self.deleting = false;
    }

    /// Move selection to the beginning of the list.
    ///
    /// During rendering this may be shifted to the second element in the list if the first (the
    /// new session candidate) is not valid.
    pub(super) fn select_first(&mut self) {
        self.list.select_first();
    }

    /// Move selection to the end of the list.
    pub(super) fn select_last(&mut self) {
        self.list.select_last();
    }

    /// Move selection down by one, unless already at the end of the list.
    pub(super) fn select_next(&mut self) {
        self.list.select_next();
    }

    /// Move selection up by one, unless already at the start of the list.
    pub(super) fn select_previous(&mut self) {
        self.list.select_previous();
    }

    /// A reference to the currently selected session, if there is one.
    pub(super) fn selected(&self) -> Option<&Session> {
        self.selected.as_ref()
    }

    /// Mark the selected session for deletion.
    pub(super) fn start_delete(&mut self) {
        self.deleting = true;
    }

    /// Take the selected session.
    ///
    /// Subsequent calls will return `None` until the next `draw` which will replenish this value.
    pub(super) fn take_selected(&mut self) -> Option<Session> {
        self.selected.take()
    }
}

/// If `text` were laid out on a single line, calculate the column that would contain the glyph
/// after the glyph corresponding to the character at `offset` (an offset in terms of code points,
/// not bytes).
fn right_margin(text: Utf32Str<'_>, offset: u32) -> u16 {
    let matched: String = text.chars().take(offset as usize + 1).collect();
    matched.width() as u16
}
