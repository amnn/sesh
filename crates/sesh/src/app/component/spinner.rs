// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Widget for representing an animated spinner.

use std::time::Duration;
use std::time::Instant;

use ratatui::prelude::Buffer;
use ratatui::prelude::Rect;
use ratatui::widgets::StatefulWidget;

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
const FRAME_DURATION: Duration = Duration::from_millis(100);

/// An animated spinner.
pub(crate) struct Spinner(bool);

/// The state of the spinner. This remembers when the animation started. Animation duration and
/// therefore frame calculation is based on this start time.
pub(crate) struct State {
    start: Instant,
}

impl Spinner {
    /// Create a spinner, enabled only when `enabled` is true.
    pub(crate) fn new(enabled: bool) -> Self {
        Self(enabled)
    }
}

impl State {
    /// Create a fresh spinner state, for an inactive spinner.
    pub(crate) fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl StatefulWidget for Spinner {
    type State = State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = area.intersection(buf.area);
        if area.is_empty() {
            return;
        }

        let Some(cell) = buf.cell_mut(area) else {
            return;
        };

        if !self.0 {
            cell.set_symbol(" ");
            return;
        }

        let now = Instant::now();
        let delta = (now - state.start).as_millis() / FRAME_DURATION.as_millis();
        let frame = (delta as usize) % FRAMES.len();
        cell.set_symbol(FRAMES[frame]);
    }
}
