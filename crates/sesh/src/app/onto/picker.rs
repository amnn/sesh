// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Rendering for the `onto` revision picker.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::StatefulWidget;

use crate::app::component::scroll;

/// Picker view over renderable log text for the current repo context.
pub(super) struct Picker {
    log: scroll::Scroll,
}

/// Mutable state owned by the onto-picker preview surface.
#[derive(Default)]
pub(super) struct State {
    scroll: scroll::State,
}

impl Picker {
    /// Create a picker view over renderable `jj log` text.
    pub(super) fn new(text: Text<'static>) -> Self {
        Self {
            log: scroll::Scroll::new(text),
        }
    }
}

impl StatefulWidget for &Picker {
    type State = State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.log.render(area, buf, &mut state.scroll);
    }
}
