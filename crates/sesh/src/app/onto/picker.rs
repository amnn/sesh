// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Rendering for the `onto` revision picker.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Text;

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

    /// Render the picker into `area` using log text as its source.
    pub(super) fn draw(&self, f: &mut Frame<'_>, area: Rect, state: &mut State) {
        f.render_stateful_widget(&self.log, area, &mut state.scroll);
    }
}
