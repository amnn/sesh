// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Widget for representing an animated spinner.

use std::time::Duration;
use std::time::Instant;

use ratatui::prelude::Buffer;
use ratatui::prelude::Rect;
use ratatui::widgets::StatefulWidget;

const DISPLAY_DELAY: Duration = Duration::from_millis(500);
const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
const FRAME_DURATION: Duration = Duration::from_millis(100);

/// An animated spinner.
pub(crate) struct Spinner(bool);

/// The state of the spinner, including when its current active period started.
pub(crate) struct State {
    started_at: Option<Instant>,
}

impl Spinner {
    /// Create a spinner, enabled only when `enabled` is true.
    pub(crate) fn new(enabled: bool) -> Self {
        Self(enabled)
    }

    /// Render after the active period reaches the display delay, returning whether it was shown.
    pub(crate) fn render_at(
        self,
        now: Instant,
        area: Rect,
        buf: &mut Buffer,
        state: &mut State,
    ) -> bool {
        if !state.show(self.0, now) {
            return false;
        }

        let area = area.intersection(buf.area);
        if area.is_empty() {
            return false;
        }

        let Some(cell) = buf.cell_mut(area) else {
            return false;
        };

        cell.set_symbol(state.frame(now, FRAME_DURATION, FRAMES));
        true
    }
}

impl State {
    /// Create a fresh spinner state, for an inactive spinner.
    pub(crate) fn new() -> Self {
        Self { started_at: None }
    }

    /// Pick the frame for instant `now` from `frames`.
    ///
    /// `frame_duration` must be at least one millisecond and `frames` must not be empty.
    pub(crate) fn frame<'f>(
        &self,
        now: Instant,
        frame_duration: Duration,
        frames: &[&'f str],
    ) -> &'f str {
        let started_at = self.started_at.unwrap_or(now);
        let delta = now.saturating_duration_since(started_at);
        let index = (delta.as_millis() / frame_duration.as_millis()) as usize;
        frames[index % frames.len()]
    }

    /// Update the active period and return whether its display delay has elapsed.
    fn show(&mut self, enabled: bool, now: Instant) -> bool {
        if !enabled {
            self.started_at = None;
            return false;
        }

        let started_at = self.started_at.get_or_insert(now);
        now.saturating_duration_since(*started_at) >= DISPLAY_DELAY
    }
}

impl StatefulWidget for Spinner {
    type State = State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.render_at(Instant::now(), area, buf, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delays_display_until_threshold() {
        let started_at = Instant::now();
        let area = Rect::new(0, 0, 6, 1);
        let mut buf = Buffer::with_lines(["header"]);
        let mut state = State::new();

        assert!(!Spinner::new(true).render_at(started_at, area, &mut buf, &mut state));
        assert!(!Spinner::new(true).render_at(
            started_at + DISPLAY_DELAY - Duration::from_millis(1),
            area,
            &mut buf,
            &mut state,
        ));
        assert_eq!(symbols(&buf), "header");

        assert!(Spinner::new(true).render_at(
            started_at + DISPLAY_DELAY,
            area,
            &mut buf,
            &mut state,
        ));
        assert_eq!(symbols(&buf), "⠴eader");
    }

    #[test]
    fn restarts_delay_after_becoming_inactive() {
        let started_at = Instant::now();
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        let mut state = State::new();

        assert!(!Spinner::new(true).render_at(started_at, area, &mut buf, &mut state));
        assert!(!Spinner::new(false).render_at(
            started_at + DISPLAY_DELAY,
            area,
            &mut buf,
            &mut state,
        ));
        assert!(!Spinner::new(true).render_at(
            started_at + DISPLAY_DELAY,
            area,
            &mut buf,
            &mut state,
        ));
    }

    fn symbols(buf: &Buffer) -> String {
        buf.content().iter().map(|cell| cell.symbol()).collect()
    }
}
