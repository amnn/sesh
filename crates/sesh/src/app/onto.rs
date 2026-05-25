// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! State for `onto` revision selection mode.

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

/// Result of handling a key while `onto` revision selection is active.
pub(super) enum Action {
    /// Leave `onto` revision selection mode.
    Cancel,
}

/// Query state for `onto` revision selection.
#[derive(Default)]
pub(super) struct State {
    query: String,
}

impl State {
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
