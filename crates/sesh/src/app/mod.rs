// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Picker UI state, rendering, and input handling.

mod block;
mod layout;
mod loading;
mod preview;
mod prompt;
mod scrollbar;
mod sessions;

use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use nucleo::Snapshot;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Widget;

use crate::app::block::Block;
use crate::app::loading::Loading;
use crate::app::preview::Preview;
use crate::app::sessions::Sessions;
use crate::picker::Picker;
use crate::session::Session;
use crate::terminal::AlternateScreenGuard;
use crate::ui::push_repo_path_spans;
use crate::ui::push_shortcut_span;

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
    session_close: bool,
}

impl App {
    /// Construct application state for the provided repo context.
    pub fn new(sessions: Vec<Session>, repo: Option<PathBuf>) -> Self {
        let load = loading::State::new();
        let picker = Picker::new(sessions.clone());
        let preview = preview::State::new(sessions);
        let sessions = sessions::State::new();
        let session_close = false;

        Self {
            load,
            picker,
            preview,
            repo,
            sessions,
            session_close,
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

        // The tool supports creating a new session if the query is non-empty and does not match
        // any live session.
        let new_valid = !query.is_empty()
            && !items
                .iter()
                .any(|i| i.data.is_tmux() && i.data.name() == query);

        let new = new_valid.then(|| Session::new(query.to_owned(), self.repo.clone()));

        Sessions::new(new, &items).draw(f, l.sessions, l.scroll, &mut self.sessions);

        // The tool supports closing the current session if it is a live tmux session.
        self.session_close = self.sessions.selected().is_some_and(Session::is_tmux);

        f.render_widget(prompt::widget(query), l.prompt);
        f.render_stateful_widget(Loading::new(status.running), l.loading, &mut self.load);

        f.render_widget(
            header_widget(snapshot, self.repo.as_deref(), self.session_close),
            l.header,
        );

        let Some(l_preview) = l.preview else {
            return;
        };

        if let Some(separator) = l.separator {
            f.render_widget(Block::new('─'), separator);
        }

        Preview::new(self.sessions.preview()).draw(f, l_preview, &mut self.preview);
    }

    /// Handle a single keyboard event, returning the consequent application action.
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        use KeyCode as KC;
        use KeyModifiers as KM;
        const CTRL: KM = KM::CONTROL;

        match key.code {
            // Accept the selected row.
            KC::Enter => return self.sessions.take_selected().map(Action::Switch),

            // Cancel
            KC::Esc => return Some(Action::Cancel),
            KC::Char('g' | 'c') if key.modifiers.contains(CTRL) => return Some(Action::Cancel),

            // Session actions
            KC::Char('x') if key.modifiers.contains(CTRL) && self.session_close => {
                return self.sessions.take_selected().map(Action::Close);
            }

            // Scroll preview
            KC::Up if key.modifiers.contains(KM::SHIFT) => {
                self.preview.scroll_up();
            }

            KC::Down if key.modifiers.contains(KM::SHIFT) => {
                self.preview.scroll_down();
            }

            // Session list selection
            KC::Up if key.modifiers.contains(KM::ALT) => {
                self.sessions.select_first();
                self.preview.first();
            }

            KC::Down if key.modifiers.contains(KM::ALT) => {
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

/// Build the header widget with match counts and current repo context.
fn header_widget(
    snapshot: &Snapshot<Session>,
    repo: Option<&Path>,
    close_session: bool,
) -> impl Widget {
    let found = snapshot.matched_items(..).count();
    let total = snapshot.item_count();
    let width = if total == 0 {
        1
    } else {
        total.ilog10() as usize + 1
    };

    let mut line = Line::default();
    let dim = Style::new().dim();

    line += Span::raw(format!(" {found:>width$}"));
    line += Span::styled(format!("/{total} | "), dim);
    push_shortcut_span(&mut line, "C-r");
    line += Span::raw(" repo: ");

    if let Some(repo) = repo {
        push_repo_path_spans(&mut line, repo);
    } else {
        line += Span::styled("none", dim);
    }

    if close_session {
        line += Span::styled(" | ", dim);
        push_shortcut_span(&mut line, "C-x");
        line += Span::raw(" close");
    }

    line
}
