// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Retained component for views loaded by a background task.

use std::future::Future;
use std::sync::Arc;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;
use tokio::sync::OnceCell;
use tokio_util::task::AbortOnDropHandle;

/// Retained background loader for a view of type `V`.
pub(crate) struct Loader<V> {
    result: Arc<OnceCell<anyhow::Result<V>>>,
    worker: AbortOnDropHandle<()>,
}

impl<V> Loader<V>
where
    V: Send + Sync + 'static,
{
    /// Start loading a view in the background.
    pub(crate) fn new<F>(load: F) -> Self
    where
        F: Future<Output = anyhow::Result<V>> + Send + 'static,
    {
        let result = Arc::new(OnceCell::new());
        let worker_result = result.clone();
        let worker = tokio::task::spawn(async move {
            let _ = worker_result.set(load.await);
        });

        Self {
            result,
            worker: AbortOnDropHandle::new(worker),
        }
    }

    /// Render the loaded view if it is ready; otherwise render a loading or error fallback.
    fn render_ready(
        &self,
        area: Rect,
        buf: &mut Buffer,
        render: impl FnOnce(&V, Rect, &mut Buffer),
    ) {
        match self.result.get() {
            Some(Ok(view)) => render(view, area, buf),
            Some(Err(err)) => format!("Error: {err}").render(area, buf),
            None if self.worker.is_finished() => "Loading failed!".render(area, buf),
            None => "Loading...".render(area, buf),
        }
    }
}

impl<V, S> StatefulWidget for &Loader<V>
where
    V: Send + Sync + 'static,
    for<'a> &'a V: StatefulWidget<State = S>,
{
    type State = S;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.render_ready(area, buf, |view, area, buf| {
            view.render(area, buf, state);
        });
    }
}

impl<V> Widget for &Loader<V>
where
    V: Send + Sync + 'static,
    for<'a> &'a V: Widget,
{
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_ready(area, buf, |view, area, buf| view.render(area, buf));
    }
}
