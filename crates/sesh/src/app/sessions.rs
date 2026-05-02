// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Component and state for rendering the session list.

use nucleo::Item;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::HighlightSpacing;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::ScrollbarState;

use crate::app::scrollbar;
use crate::picker::Item as _;
use crate::session::Session;

pub(super) struct Sessions<'s> {
    new: Option<Session>,
    rest: &'s [Item<'s, Session>],
}

#[derive(Default)]
pub(super) struct State {
    list: ListState,
    selected: Option<Session>,
}

impl<'s> Sessions<'s> {
    /// Create a new `Session` component with `new` representing the potential new session, and
    /// `rest` being the other candidates.
    pub(super) fn new(new: Option<Session>, rest: &'s [Item<'s, Session>]) -> Self {
        Self { new, rest }
    }

    pub(super) fn draw(&self, f: &mut Frame<'_>, list: Rect, scroll: Rect, state: &mut State) {
        let mut rows = Vec::with_capacity(self.rest.len() + 1);

        state.selected = match (state.list.selected_mut(), &self.new, &self.rest[..]) {
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

            // Otherwise, if this is no selection, default to the first `rest` session.
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
            rows.push(session.render(selected == Some(0)))
        } else {
            rows.push(ListItem::new(""))
        }

        for (i, item) in (1..).zip(self.rest) {
            rows.push(item.data.render(selected == Some(i)))
        }

        let height = list.height as usize;
        let mut scroll_state = ScrollbarState::default()
            .content_length(rows.len().saturating_sub(height) + 1)
            .viewport_content_length(height)
            .position(state.list.offset());

        let sessions = List::new(rows)
            .highlight_symbol(Span::styled("▌", Style::new().bg(Color::Red)))
            .highlight_spacing(HighlightSpacing::Always);

        f.render_stateful_widget(sessions, list, &mut state.list);
        f.render_stateful_widget(scrollbar::widget(), scroll, &mut scroll_state);
    }
}

impl State {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// The session to preview. This is similar to [`State::selected`], but if the currently
    /// selected session is the new one, then that also returns `None`, as there will not be a
    /// prepared preview for this session.
    pub(super) fn preview(&self) -> Option<&Session> {
        match self.list.selected() {
            None | Some(0) => None,
            Some(_) => self.selected.as_ref(),
        }
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

    /// Take the selected session.
    ///
    /// Subsequent calls will return `None` until the next `draw` which will replenish this value.
    pub(super) fn take_selected(&mut self) -> Option<Session> {
        self.selected.take()
    }
}
