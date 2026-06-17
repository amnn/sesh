// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! State for `onto` revision selection mode.

mod picker;

use std::path::PathBuf;

use ansi_to_tui::IntoText as _;
use anyhow::Context as _;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Text;
use tokio::sync::oneshot;
use tokio::sync::oneshot::error::TryRecvError;
use tokio_util::task::AbortOnDropHandle;

use crate::app::onto::picker::Picker;
use crate::cmd::jj;

/// Result of handling a key while `onto` revision selection is active.
pub(super) enum Action {
    /// Leave `onto` revision selection mode.
    Cancel,
}

/// Query, picker, and loading state for `onto` revision selection.
pub(super) struct State {
    picker: picker::State,
    query: String,
    pick_rx: oneshot::Receiver<Picker>,
    view: Option<Picker>,
    _worker: AbortOnDropHandle<()>,
}

impl State {
    /// Create onto-selection state and start loading the current repo's log output.
    pub(super) fn new(repo: PathBuf) -> Self {
        let (tx, pick_rx) = oneshot::channel();
        let worker = tokio::task::spawn(async move {
            let text = jj::log(&repo)
                .await
                .with_context(|| {
                    format!("failed to build onto picker for repo '{}'", repo.display())
                })
                .and_then(|log| {
                    log.into_bytes()
                        .into_text()
                        .context("failed to render jj log output")
                })
                .unwrap_or_else(|err| Text::raw(format!("Error: {err}")));

            tx.send(Picker::new(text)).ok();
        });

        Self {
            picker: picker::State::default(),
            query: String::new(),
            pick_rx,
            view: None,
            _worker: AbortOnDropHandle::new(worker),
        }
    }

    /// Render the onto picker into `area`.
    pub(super) fn draw(&mut self, f: &mut Frame<'_>, area: Rect) {
        if let Some(view) = &mut self.view {
            view.draw(f, area, &mut self.picker);
            return;
        }

        match self.pick_rx.try_recv() {
            Ok(mut view) => {
                view.draw(f, area, &mut self.picker);
                self.view = Some(view);
            }

            Err(TryRecvError::Empty) => {
                f.render_widget("Loading...", area);
            }

            Err(TryRecvError::Closed) => {
                f.render_widget("Error: onto picker worker stopped", area);
            }
        }
    }

    /// Handle a key event while `onto` revision selection mode is active.
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        use KeyCode as KC;
        use KeyModifiers as KM;

        const CTRL: KM = KM::CONTROL;
        const SHIFT: KM = KM::SHIFT;

        match key.code {
            // Cancel
            KC::Esc => return Some(Action::Cancel),
            KC::Char('c' | 'g' | 'o') if key.modifiers.contains(CTRL) => {
                return Some(Action::Cancel);
            }

            // Edit query
            KC::Backspace => self.pop_query(),
            KC::Char('u') if key.modifiers.contains(CTRL) => self.clear_query(),
            KC::Char(c) if key.modifiers.is_empty() => self.push_query(c),
            KC::Char(c) if key.modifiers.contains(SHIFT) => self.push_query(c),

            _ => {}
        }

        None
    }

    /// Return the current `onto` revision query.
    pub(super) fn query(&self) -> &str {
        &self.query
    }

    /// Clear the `onto` revision query.
    fn clear_query(&mut self) {
        self.query.clear();
    }

    /// Remove the final character from the `onto` revision query.
    fn pop_query(&mut self) {
        self.query.pop();
    }

    /// Append `c` to the `onto` revision query.
    fn push_query(&mut self, c: char) {
        self.query.push(c);
    }
}
