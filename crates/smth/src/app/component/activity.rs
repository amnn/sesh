// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Progress widget and retained state for a long-running background activity.

use std::future::Future;
use std::marker::PhantomData;
use std::time::Duration;
use std::time::Instant;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget as _;

use crate::app::component::loader;
use crate::app::component::spinner;
use crate::app::component::spinner::Spinner;

/// Expanding and contracting dots with one trailing padding cell for the header overdraw.
const FRAMES: &[&str] = &[". ", ".. ", "... ", ".. "];
const FRAME_DURATION: Duration = Duration::from_millis(250);
const PREFIX_WIDTH: u16 = 2;

/// Progress widget for a background activity.
pub(crate) struct Activity<V>(PhantomData<fn() -> V>);

/// Retained state for a background activity with owner-styled progress text.
pub(crate) struct State<V> {
    loader: loader::State<V>,
    label: Span<'static>,
    spinner: spinner::State,
}

impl<V> Activity<V> {
    /// Create an activity progress widget.
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<V> State<V>
where
    V: Send + 'static,
{
    /// Start an activity whose progress uses `label`, including its ratatui style.
    pub(crate) fn new<F>(label: impl Into<Span<'static>>, load: F) -> Self
    where
        F: Future<Output = anyhow::Result<V>> + Send + 'static,
    {
        Self {
            loader: loader::State::new(load),
            label: label.into(),
            spinner: spinner::State::new(),
        }
    }
}

impl<V> State<V> {
    /// Return whether the activity was loading when its state was last updated.
    pub(crate) fn is_loading(&self) -> bool {
        self.loader.is_loading()
    }

    /// Take the activity result once its background task has completed.
    ///
    /// Returns `None` while the task is still loading or after its result has already been taken.
    pub(crate) fn take(&mut self) -> Option<anyhow::Result<V>> {
        self.loader.take()
    }
}

impl<V> StatefulWidget for Activity<V> {
    type State = State<V>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.loader.poll();

        let area = area.intersection(buf.area);
        let is_loading = state.is_loading();
        let now = Instant::now();
        if !Spinner::new(is_loading).render_at(now, area, buf, &mut state.spinner) {
            return;
        }

        let offset = area.width.min(PREFIX_WIDTH);
        if offset == PREFIX_WIDTH {
            " ".render(Rect::new(area.x + 1, area.y, 1, area.height), buf);
        }

        let label = Rect::new(area.x + offset, area.y, area.width - offset, area.height);
        let ellipsis = Span::styled(
            state.spinner.frame(now, FRAME_DURATION, FRAMES),
            state.label.style,
        );
        let text = Line::from(vec![state.label.clone(), ellipsis]);
        text.render(label, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_frames_only_cover_dots_and_one_padding_cell() {
        for frame in FRAMES {
            let dots = frame
                .strip_suffix(' ')
                .expect("frame should end with one padding cell");
            assert!(!dots.is_empty());
            assert!(dots.chars().all(|ch| ch == '.'));
            assert_eq!(frame.chars().count(), dots.chars().count() + 1);
        }
    }
}
