// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Picker UI state, rendering, and input handling.

use std::collections::HashMap;
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
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::text::Line;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;

use crate::path::TruncatedExt as _;
use crate::picker::Picker;
use crate::session::Session;
use crate::terminal::AlternateScreenGuard;

const POLL_TIMEOUT: Duration = Duration::from_millis(16);

/// Session picker state, caches, and UI behavior.
pub struct App {
    current_repo: Option<PathBuf>,
    preview_cache: HashMap<Session, String>,
    query: String,
    selected: usize,
    scroll: usize,
    visible_items: Vec<Session>,
}

impl App {
    /// Construct application state for the provided repo context.
    pub(crate) fn new(current_repo: Option<PathBuf>) -> Self {
        Self {
            current_repo,
            preview_cache: HashMap::new(),
            query: String::new(),
            selected: 0,
            scroll: 0,
            visible_items: Vec::new(),
        }
    }

    /// Build the header text shown above the picker.
    pub(crate) fn header(&self) -> String {
        match self.current_repo.as_deref() {
            Some(repo) => format!("Current repo: {}", repo.truncated()),
            None => "Current repo: none".to_owned(),
        }
    }

    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    pub(crate) fn clear_query(&mut self) {
        if self.query.is_empty() {
            return;
        }

        self.query.clear();
    }

    pub(crate) fn move_down(&mut self) {
        if self.selected + 1 < self.visible_items.len() {
            self.selected += 1;
        }
    }

    pub(crate) fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub(crate) fn pop_query(&mut self) -> bool {
        self.query.pop().is_some()
    }

    /// Return the preview for the selected session, populating the cache on demand.
    pub(crate) fn preview_text(&mut self, width: usize) -> String {
        let Some(session) = self.selected_session() else {
            return String::new();
        };

        if let Some(preview) = self.preview_cache.get(session) {
            return preview.clone();
        }

        let preview = match session.preview(width) {
            Ok(preview) => strip_ansi(&preview),
            Err(error) => format!("Failed to render preview: {error:?}"),
        };
        self.preview_cache.insert(session.clone(), preview.clone());
        preview
    }

    /// Replace the visible rows after a matcher refresh and preserve selection when possible.
    pub(crate) fn replace_visible_items(&mut self, items: Vec<Session>) {
        let previous = self.selected_session().cloned();
        self.visible_items = items;
        self.selected = selected_row(&self.visible_items, previous.as_ref(), self.selected);
    }

    /// Return the currently selected session, if any.
    pub(crate) fn selected_session(&self) -> Option<&Session> {
        self.visible_items.get(self.selected).map(|session| session)
    }

    /// Update the current repo context from the selected session.
    pub(crate) fn set_current_repo_from_selection(&mut self) {
        let Some(repo) = self.selected_session().and_then(Session::repo) else {
            return;
        };

        self.current_repo = Some(repo_context_path(repo));
    }

    /// Replace the current query string.
    pub(crate) fn set_query(&mut self, query: String) {
        self.query = query;
    }

    /// Render the picker UI for the current frame.
    pub(crate) fn draw(&mut self, frame: &mut ratatui::Frame<'_>, total_items: usize) {
        let areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(frame.area());
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(areas[0]);

        frame.render_widget(Paragraph::new(format!("Session: {}", self.query)), left[0]);
        frame.render_widget(
            Paragraph::new(format_status_line(
                left[1].width as usize,
                self.visible_items.len(),
                total_items,
            )),
            left[1],
        );
        frame.render_widget(Paragraph::new(format!("  {}", self.header())), left[2]);

        let list_height = left[3].height as usize;
        self.scroll = scroll_offset(
            self.scroll,
            self.selected,
            self.visible_items.len(),
            list_height,
        );
        let lines = self
            .visible_items
            .iter()
            .enumerate()
            .skip(self.scroll)
            .take(list_height)
            .map(|(index, session)| {
                let prefix = if index == self.selected { "> " } else { "  " };
                Line::from(format!("{prefix}{}", session.item()))
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(Text::from(lines)), left[3]);

        let preview = self.preview_text(areas[1].width as usize);
        frame.render_widget(Paragraph::new(preview), areas[1]);
    }

    /// Handle a single keyboard event, returning `true` when the picker should exit.
    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => true,
            KeyCode::Up => {
                self.move_up();
                false
            }
            KeyCode::Down => {
                self.move_down();
                false
            }
            KeyCode::Backspace => {
                self.pop_query();
                false
            }
            KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_down();
                false
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_up();
                false
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.set_current_repo_from_selection();
                false
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.clear_query();
                false
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                let mut query = self.query.clone();
                query.push(c);
                self.set_query(query);
                false
            }
            _ => false,
        }
    }

    /// Run the interactive picker for discovered sessions.
    pub fn run(sessions: Vec<Session>, current_repo: Option<PathBuf>) -> anyhow::Result<()> {
        let _guard = AlternateScreenGuard::new()?;
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let mut app = App::new(current_repo);
        let mut picker = Picker::new(sessions);

        loop {
            picker.refresh_matches(&mut app);
            terminal.draw(|frame| app.draw(frame, picker.total_items()))?;

            if !event::poll(POLL_TIMEOUT)? {
                continue;
            }

            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let previous_query = app.query().to_owned();
                    if app.handle_key(key) {
                        break;
                    }
                    if app.query() != previous_query {
                        picker.set_query(&previous_query, app.query());
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        Ok(())
    }
}

/// Format the matcher status line for the current terminal width.
fn format_status_line(width: usize, matched: usize, total: usize) -> String {
    let left = format!("  {matched}/{total}");
    let right = "0/0";
    if width <= left.len() + 1 + right.len() {
        return format!("{left} {right}");
    }

    format!("{left}{:>padding$}", right, padding = width - left.len())
}

/// Normalize a selected repo path before storing it in the UI state.
fn repo_context_path(repo: &Path) -> PathBuf {
    repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf())
}

/// Keep the selected row visible within a scrollable list viewport.
fn scroll_offset(current: usize, selected: usize, len: usize, height: usize) -> usize {
    if height == 0 || len == 0 {
        return 0;
    }

    let max_offset = len.saturating_sub(height);
    if selected < current {
        selected
    } else if selected >= current + height {
        (selected + 1).saturating_sub(height).min(max_offset)
    } else {
        current.min(max_offset)
    }
}

/// Preserve the previous selection when that session remains visible.
fn selected_row(items: &[Session], previous: Option<&Session>, selected: usize) -> usize {
    if items.is_empty() {
        return 0;
    }

    previous
        .and_then(|previous| items.iter().position(|session| session == previous))
        .unwrap_or_else(|| selected.min(items.len() - 1))
}

/// Remove ANSI escape sequences from terminal output.
fn strip_ansi(text: &str) -> String {
    let mut stripped = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            stripped.push(ch);
            continue;
        }

        if chars.next_if_eq(&'[').is_none() {
            continue;
        }

        for next in chars.by_ref() {
            if ('@'..='~').contains(&next) {
                break;
            }
        }
    }

    stripped
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn canonicalizes_repo_context_path() {
        let temp = tempdir().unwrap();
        let repo = temp.path().join("repo");
        fs::create_dir(&repo).unwrap();

        let relative = repo.strip_prefix(temp.path()).unwrap();
        let cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        assert_eq!(repo_context_path(relative), repo.canonicalize().unwrap());

        std::env::set_current_dir(cwd).unwrap();
    }

    #[test]
    fn preserves_selected_row_when_item_is_still_visible() {
        let previous = Session::from_repo(PathBuf::from("/tmp/beta")).unwrap();
        let items = vec![
            Session::from_repo(PathBuf::from("/tmp/alpha")).unwrap(),
            previous.clone(),
        ];

        assert_eq!(selected_row(&items, Some(&previous), 0), 1);
    }

    #[test]
    fn renders_header_with_current_repo() {
        let app = App::new(Some(PathBuf::from("/tmp/repo")));

        assert_eq!(app.header(), "Current repo: /tmp/repo");
    }

    #[test]
    fn renders_header_without_current_repo() {
        let app = App::new(None);

        assert_eq!(app.header(), "Current repo: none");
    }

    #[test]
    fn strips_ansi_escape_sequences() {
        assert_eq!(strip_ansi("\u{1b}[31mhello\u{1b}[0m"), "hello");
    }
}
