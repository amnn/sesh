// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Picker UI state, rendering, and input handling.

mod block;
mod header;
mod layout;
mod loading;
mod preview;
mod prompt;
mod scrollbar;
mod sessions;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use nucleo::Item;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::block::Block;
use crate::app::header::Header;
use crate::app::loading::Loading;
use crate::app::preview::Preview;
use crate::app::sessions::Sessions;
use crate::picker::Picker;
use crate::session::Session;
use crate::terminal::AlternateScreenGuard;

/// Timeout for waiting for a key event.
const POLL_TIMEOUT: Duration = Duration::from_millis(16);

/// Completed action chosen from the picker.
pub enum Action {
    /// Do nothing and exit the picker.
    Cancel,

    /// Kill the selected live tmux session.
    Close(Session),

    /// Switch to the selected session, creating it first if needed.
    Switch(Session),
}

/// Session picker state, caches, and UI behavior.
pub struct App {
    load: loading::State,
    picker: Picker<Session>,
    preview: preview::State,
    repo: Option<PathBuf>,
    sessions: sessions::State,
}

impl App {
    /// Construct application state for the provided repo context.
    pub fn new(sessions: Vec<Session>, repo: Option<PathBuf>) -> Self {
        let load = loading::State::new();
        let picker = Picker::new(sessions.clone());
        let preview = preview::State::new(sessions);
        let sessions = sessions::State::new();

        Self {
            load,
            picker,
            preview,
            repo,
            sessions,
        }
    }

    /// Run the interactive picker for discovered sessions.
    pub fn run(mut self) -> anyhow::Result<Action> {
        let _guard = AlternateScreenGuard::new()?;
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if !event::poll(POLL_TIMEOUT)? {
                continue;
            }

            let Event::Key(key) = event::read()? else {
                continue;
            };

            if key.kind != KeyEventKind::Press {
                continue;
            }

            if let Some(action) = self.handle_key(key) {
                return Ok(action);
            }
        }
    }

    /// Draw the UI into the provided frame based on the current application state.
    ///
    /// The frame is split up into regions, each with its own widget. The `preview` region and its
    /// scroll bar are only visible when the preview is toggled on (defaults to visible).
    fn draw(&mut self, f: &mut ratatui::Frame<'_>) {
        let l = layout::Layout::new(f.area(), self.preview.visible());

        // Poll the picker for its latest state, and build the data model.
        let (status, snapshot, query) = self.picker.refresh();
        let items: Vec<_> = snapshot.matched_items(..).collect();

        // (1) Render picker state
        f.render_widget(prompt::widget(query), l.prompt);
        f.render_stateful_widget(Loading::new(status.running), l.loading, &mut self.load);

        let can_new = !name_collision(query, &items);
        let sessions = Sessions::new(
            can_new.then(|| Session::new(query.to_owned(), self.repo.clone())),
            &items,
            snapshot.pattern().column_pattern(0),
        );

        // (2) Render session list. This also updates `self.sessions`, so that the selected index
        // and session are up-to-date and valid.
        sessions.draw(f, l.sessions, l.scroll, &mut self.sessions);

        let header = Header::new(
            self.sessions.can_close(),
            items.len(),
            self.repo.as_deref(),
            snapshot.item_count() as usize,
        );

        // (3) Render the header, which depends on the currently selected session (so must happen
        // after session list rendering).
        header.draw(f, l.header);

        let Some(l_preview) = l.preview else {
            return;
        };

        if let Some(separator) = l.separator {
            f.render_widget(Block::new('─'), separator);
        }

        let preview = Preview::new(self.sessions.preview());

        // (4) Render the preview, if it is toggled on. This also depends on whatever the currently
        // selected session is, so it must be rendered after the session list.
        preview.draw(f, l_preview, &mut self.preview);
    }

    /// Handle a single keyboard event, returning the consequent application action.
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        use KeyCode as KC;
        use KeyModifiers as KM;

        const ALT: KM = KM::ALT;
        const CTRL: KM = KM::CONTROL;
        const SHIFT: KM = KM::SHIFT;

        match key.code {
            // Accept the selected row.
            KC::Enter => return self.sessions.take_selected().map(Action::Switch),

            // Cancel
            KC::Esc => return Some(Action::Cancel),
            KC::Char('g' | 'c') if key.modifiers.contains(CTRL) => return Some(Action::Cancel),

            // Session actions
            KC::Char('x') if key.modifiers.contains(CTRL) && self.sessions.can_close() => {
                return self.sessions.take_selected().map(Action::Close);
            }

            // Scroll preview
            KC::Up if key.modifiers.contains(SHIFT) => {
                self.preview.scroll_up();
            }

            KC::Down if key.modifiers.contains(SHIFT) => {
                self.preview.scroll_down();
            }

            // Session list selection
            KC::Up if key.modifiers.contains(ALT) => {
                self.sessions.select_first();
                self.preview.first();
            }

            KC::Down if key.modifiers.contains(ALT) => {
                self.sessions.select_last();
                self.preview.first();
            }

            KC::Up => {
                self.sessions.select_previous();
                self.preview.first();
            }

            KC::Down => {
                self.sessions.select_next();
                self.preview.first();
            }

            // App state
            KC::Char('r') if key.modifiers.contains(ALT) => self.reset_current_repo(),
            KC::Char('r') if key.modifiers.contains(CTRL) => self.set_current_repo(),

            // View state
            KC::Char('p') if key.modifiers.contains(CTRL) => {
                self.preview.toggle();
            }

            // Edit query
            KC::Backspace => self.picker.pop(),
            KC::Char('u') if key.modifiers.contains(CTRL) => self.picker.clear(),
            KC::Char(c) if key.modifiers.is_empty() => self.picker.push(c),

            _ => {}
        };

        None
    }

    /// Clear the current repo.
    fn reset_current_repo(&mut self) {
        self.repo = None;
    }

    /// Set the current repo from the currently selected session.
    ///
    /// If there is no selection, or the selected session has no associated repo, the current repo
    /// is cleared.
    fn set_current_repo(&mut self) {
        self.repo = self
            .sessions
            .selected()
            .and_then(|s| s.repo().map(|p| p.to_owned()));
    }
}

/// Whether `name` collides with the candidate sessions.
///
/// Names can collide if they are empty, or if they match the name of an existing tmux session.
fn name_collision(name: &str, items: &[Item<'_, Session>]) -> bool {
    name.is_empty()
        || items
            .iter()
            .any(|i| i.data.is_tmux() && i.data.name() == name)
}
