// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Widget for representing an animated loading spinner.

use std::time::Duration;
use std::time::Instant;

use ratatui::prelude::Buffer;
use ratatui::prelude::Rect;
use ratatui::widgets::StatefulWidget;

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
const FRAME_DURATION: Duration = Duration::from_millis(100);

/// An animated loading spinner.
pub(crate) struct Loading(pub bool);

/// The state of the loading spinner. This remembers when the animation started. Animation duration
/// and therefore frame calculation is based on this start time.
pub(crate) struct LoadingState {
    start: Instant,
}

impl LoadingState {
    /// Create a fresh loading state, for an inactive loading spinner.
    pub(crate) fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl StatefulWidget for Loading {
    type State = LoadingState;

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
