// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Terminal state management helpers.

use crossterm::cursor;
use crossterm::execute;
use crossterm::terminal;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;

/// Enters alternate screen and raw mode on construction and restores the previous state on drop.
pub struct AlternateScreenGuard;

impl AlternateScreenGuard {
    /// Enter the alternate screen and enable raw terminal mode.
    pub fn new() -> anyhow::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(std::io::stdout(), EnterAlternateScreen, cursor::Hide)?;
        Ok(Self)
    }
}

impl Drop for AlternateScreenGuard {
    fn drop(&mut self) {
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen, cursor::Show);
        let _ = terminal::disable_raw_mode();
    }
}
