// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Picker UI state, rendering, and input handling.

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

use crate::cache::PreviewCache;
use crate::path::TruncatedExt as _;
use crate::picker::Item as _;
use crate::picker::Picker;
use crate::session::Session;
use crate::terminal::AlternateScreenGuard;

const POLL_TIMEOUT: Duration = Duration::from_millis(16);

/// Session picker state, caches, and UI behavior.
pub struct App {
    repo: Option<PathBuf>,
    picker: Picker<Session>,
    cache: PreviewCache<Session>,
    query: String,
    selected: usize,
    scroll: usize,
    visible_items: Vec<Session>,
}

impl App {
    /// Construct application state for the provided repo context.
    pub fn new(sessions: Vec<Session>, repo: Option<PathBuf>) -> Self {
        let picker = Picker::new(sessions.clone());
        let cache = PreviewCache::new(sessions);

        Self {
            repo,
            picker,
            cache,
            query: String::new(),
            selected: 0,
            scroll: 0,
            visible_items: vec![],
        }
    }

    /// Run the interactive picker for discovered sessions.
    pub fn run(mut self) -> anyhow::Result<()> {
        let _guard = AlternateScreenGuard::new()?;
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

        loop {
            let items = self.picker.refresh_matches().cloned().collect();
            self.replace_visible_items(items);

            let total_items = self.picker.total_items();
            terminal.draw(|frame| self.draw(frame, total_items))?;

            if !event::poll(POLL_TIMEOUT)? {
                continue;
            }

            let Event::Key(key) = event::read()? else {
                continue;
            };

            if key.kind != KeyEventKind::Press {
                continue;
            }

            let previous_query = self.query().to_owned();
            if self.handle_key(key) {
                break;
            }

            if self.query() != previous_query {
                let query = self.query().to_owned();
                self.picker.set_query(&previous_query, &query);
            }
        }

        Ok(())
    }

    /// Clear the active query string.
    pub(crate) fn clear_query(&mut self) {
        if self.query.is_empty() {
            return;
        }

        self.query.clear();
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

        let lines: Vec<_> = self
            .visible_items
            .iter()
            .enumerate()
            .skip(self.scroll)
            .take(list_height)
            .map(|(index, session)| {
                let prefix = if index == self.selected { "> " } else { "  " };
                Line::from(format!("{prefix}{}", session.text()))
            })
            .collect();

        frame.render_widget(Paragraph::new(Text::from(lines)), left[3]);

        let preview = self.preview_text();
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

    /// Build the header text shown above the picker.
    pub(crate) fn header(&self) -> String {
        match self.repo.as_deref() {
            Some(repo) => format!("Current repo: {}", repo.truncated()),
            None => "Current repo: none".to_owned(),
        }
    }

    /// Move the selection to the next visible item when possible.
    pub(crate) fn move_down(&mut self) {
        if self.selected + 1 < self.visible_items.len() {
            self.selected += 1;
        }
    }

    /// Move the selection to the previous visible item when possible.
    pub(crate) fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Remove the trailing query character and report whether one was removed.
    pub(crate) fn pop_query(&mut self) -> bool {
        self.query.pop().is_some()
    }

    /// Return the preview for the selected session from the background cache.
    pub(crate) fn preview_text(&self) -> String {
        let Some(session) = self.selected_session() else {
            return String::new();
        };

        let Some(preview) = self.cache.get(&session.text()) else {
            return "Loading preview...".to_owned();
        };

        match preview.as_ref() {
            Ok(preview) => preview.clone(),
            Err(e) => format!("Error loading preview: {e:?}"),
        }
    }

    /// Return the active query string.
    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    /// Replace the visible rows after a matcher refresh and preserve selection when possible.
    pub(crate) fn replace_visible_items(&mut self, items: Vec<Session>) {
        let previous = self.selected_session().cloned();
        self.visible_items = items;
        self.selected = selected_row(&self.visible_items, previous.as_ref(), self.selected);
    }

    /// Return the currently selected session, if any.
    pub(crate) fn selected_session(&self) -> Option<&Session> {
        self.visible_items.get(self.selected)
    }

    /// Update the current repo context from the selected session.
    pub(crate) fn set_current_repo_from_selection(&mut self) {
        let Some(repo) = self.selected_session().and_then(Session::repo) else {
            return;
        };

        self.repo = Some(repo_context_path(repo));
    }

    /// Replace the current query string.
    pub(crate) fn set_query(&mut self, query: String) {
        self.query = query;
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

    #[tokio::test]
    async fn renders_header_with_current_repo() {
        let app = App::new(vec![], Some(PathBuf::from("/tmp/repo")));

        assert_eq!(app.header(), "Current repo: /tmp/repo");
    }

    #[tokio::test]
    async fn renders_header_without_current_repo() {
        let app = App::new(vec![], None);

        assert_eq!(app.header(), "Current repo: none");
    }
}
