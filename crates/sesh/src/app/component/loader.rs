// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Stateful component for views loaded by a background task.

use std::future::Future;
use std::marker::PhantomData;

use anyhow::anyhow;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget as _;
use tokio::sync::oneshot;
use tokio::sync::oneshot::error::TryRecvError;
use tokio_util::task::AbortOnDropHandle;

/// View renderer for a background-loaded stateful view of type `V`.
pub(crate) struct Loader<'s, V, S> {
    state: &'s mut S,
    _view: PhantomData<fn() -> V>,
}

/// Retained loading state for a background-loaded view.
pub(crate) struct State<V> {
    status: Status<V>,
}

/// Current result state for a background-loaded view.
pub(crate) enum Status<V> {
    /// The background task is still running.
    Loading(Task<V>),

    /// The background task failed or ended before returning a view.
    Error(anyhow::Error),

    /// The view has loaded. `done` is true if the loaded view has been acknowledged.
    Loaded { view: V, done: bool },
}

/// In-flight background task state while a view is loading.
pub(crate) struct Task<V> {
    rx: oneshot::Receiver<anyhow::Result<V>>,
    _h: AbortOnDropHandle<()>,
}

impl<'s, V, S> Loader<'s, V, S> {
    /// Create a renderer for a background-loaded view that uses `state` for the loaded view.
    pub(crate) fn new(state: &'s mut S) -> Self {
        Self {
            state,
            _view: PhantomData,
        }
    }
}

impl<V> State<V>
where
    V: Send + 'static,
{
    /// Start loading a view in the background.
    pub(crate) fn new<F>(load: F) -> Self
    where
        F: Future<Output = anyhow::Result<V>> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let worker = tokio::task::spawn(async move {
            let _ = tx.send(load.await);
        });

        Self {
            status: Status::Loading(Task {
                rx,
                _h: AbortOnDropHandle::new(worker),
            }),
        }
    }
}

impl<V> State<V> {
    /// Mark a loaded view as handled without changing its rendered output.
    #[allow(dead_code)]
    pub(crate) fn finish(&mut self) -> bool {
        self.poll();

        use Status as S;
        match &mut self.status {
            S::Loading(_) | S::Error(_) => false,
            S::Loaded { done: true, .. } => false,
            S::Loaded { done, .. } => {
                *done = true;
                true
            }
        }
    }

    /// Poll the background task and update the status if the load completed.
    pub(crate) fn poll(&mut self) {
        match &mut self.status {
            Status::Loading(task) => match task.rx.try_recv() {
                Ok(Ok(view)) => self.status = Status::Loaded { view, done: false },
                Ok(Err(err)) => self.status = Status::Error(err),
                Err(TryRecvError::Empty) => { /* nop */ }
                Err(TryRecvError::Closed) => {
                    self.status = Status::Error(anyhow!("failed to load view"))
                }
            },
            Status::Error(_) | Status::Loaded { .. } => { /* nop */ }
        }
    }

    /// Return the view if it has loaded and has not yet been marked handled.
    #[allow(dead_code)]
    pub(crate) fn view(&mut self) -> Option<&V> {
        self.poll();

        if let Status::Loaded { view, done: false } = &self.status {
            Some(view)
        } else {
            None
        }
    }
}

impl<V, S> StatefulWidget for Loader<'_, V, S>
where
    for<'a> &'a V: StatefulWidget<State = S>,
{
    type State = State<V>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.poll();

        match &state.status {
            Status::Loading(_) => "Loading...".render(area, buf),
            Status::Error(err) => format!("Error: {err}").render(area, buf),
            Status::Loaded { view, .. } => {
                view.render(area, buf, self.state);
            }
        }
    }
}
