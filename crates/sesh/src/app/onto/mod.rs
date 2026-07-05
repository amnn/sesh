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

use crate::app::component::loader;
use crate::app::component::loader::Loader;
use crate::app::onto::picker::Candidate;
use crate::app::onto::picker::Picker;
use crate::cmd::jj;
use crate::model;

/// Result of handling a key while `onto` revision selection is active.
pub(super) enum Action {
    /// Leave `onto` revision selection mode.
    Cancel,
}

/// Query, picker, and loading state for `onto` revision selection.
pub(super) struct State {
    picker: loader::State<Picker>,
    state: picker::State,
    model: model::picker::Picker<Candidate>,
}

impl State {
    /// Create onto-selection state and start loading the current repo's log output.
    pub(super) fn new(repo: PathBuf) -> Self {
        let picker = loader::State::new(async move {
            let text = jj::log(&repo)
                .await
                .with_context(|| {
                    format!("failed to build onto picker for repo '{}'", repo.display())
                })?
                .into_bytes()
                .into_text()
                .context("failed to render jj log output")?;

            Ok(Picker::new(text))
        });

        Self {
            picker,
            state: picker::State::default(),
            model: model::picker::Picker::new(String::new()),
        }
    }

    /// Render the onto picker into `area`.
    pub(super) fn draw(&mut self, f: &mut Frame<'_>, area: Rect) {
        if let Some(picker) = self.picker.pending() {
            self.model.inject(picker.candidates());
            self.picker.finish();
        }

        f.render_stateful_widget(Loader::new(&mut self.state), area, &mut self.picker);
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
        &self.model.query()
    }

    /// Clear the `onto` revision query.
    fn clear_query(&mut self) {
        self.model.clear();
    }

    /// Remove the final character from the `onto` revision query.
    fn pop_query(&mut self) {
        self.model.pop();
    }

    /// Append `c` to the `onto` revision query.
    fn push_query(&mut self, c: char) {
        self.model.push(c);
    }
}
