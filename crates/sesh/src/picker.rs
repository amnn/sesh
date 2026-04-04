// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! RataTUI-based session picker backed by `nucleo` fuzzy matching.

use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crossterm::cursor;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use crossterm::execute;
use crossterm::terminal;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use nucleo::Config;
use nucleo::Nucleo;
use nucleo::Utf32String;
use nucleo::pattern::CaseMatching;
use nucleo::pattern::Normalization;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::text::Line;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;

use crate::path::TruncatedExt as _;
use crate::session::Session;

const MATCH_COLUMNS: u32 = 1;
const TICK_TIMEOUT_MS: u64 = 10;
const POLL_TIMEOUT: Duration = Duration::from_millis(16);

/// Shared UI state for the session picker.
#[derive(Clone, Debug, Default)]
pub struct State {
    current_repo: Option<PathBuf>,
}

/// Cached fuzzy-match input derived from a session.
#[derive(Clone, Debug)]
struct PickerItem {
    session: Session,
    text: String,
}

/// Session row currently visible in the picker list.
#[derive(Clone, Debug)]
struct VisibleItem {
    session: Session,
    text: String,
}

/// Session picker state, caches, and fuzzy matcher.
struct App {
    matcher: Nucleo<PickerItem>,
    preview_cache: HashMap<Session, String>,
    query: String,
    selected: usize,
    scroll: usize,
    state: State,
    visible_items: Vec<VisibleItem>,
}

/// Restores terminal state when the picker exits.
struct TerminalGuard;

impl State {
    /// Construct state for a picker launched from `current_repo`.
    pub fn new(current_repo: Option<PathBuf>) -> Self {
        Self { current_repo }
    }

    /// Build the header text shown above the picker.
    fn header(&self) -> String {
        match self.current_repo.as_deref() {
            Some(repo) => format!("Current repo: {}", repo.truncated()),
            None => "Current repo: none".to_owned(),
        }
    }
}

impl App {
    /// Construct application state for the provided sessions.
    fn new(sessions: Vec<Session>, state: State) -> Self {
        let matcher = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, MATCH_COLUMNS);
        let injector = matcher.injector();
        for session in sessions {
            let item = PickerItem {
                text: session.item(),
                session,
            };
            injector.push(item, |item, columns| {
                columns[0] = Utf32String::from(item.text.as_str())
            });
        }
        drop(injector);

        Self {
            matcher,
            preview_cache: HashMap::new(),
            query: String::new(),
            selected: 0,
            scroll: 0,
            state,
            visible_items: Vec::new(),
        }
    }

    fn clear_query(&mut self) {
        if self.query.is_empty() {
            return;
        }

        self.set_query(String::new());
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.visible_items.len() {
            self.selected += 1;
        }
    }

    fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn pop_query(&mut self) {
        if self.query.pop().is_some() {
            self.reparse_query(false);
        }
    }

    /// Return the preview for the selected session, populating the cache on demand.
    fn preview_text(&mut self, width: usize) -> String {
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

    /// Refresh the visible rows from the current fuzzy-match snapshot.
    fn refresh_matches(&mut self) {
        let previous = self.selected_session().cloned();
        let mut status = self.matcher.tick(TICK_TIMEOUT_MS);
        while self.matcher.snapshot().item_count() == 0 && status.running {
            status = self.matcher.tick(TICK_TIMEOUT_MS);
        }

        let snapshot = self.matcher.snapshot();
        let matched = snapshot.matched_item_count();
        self.visible_items = snapshot
            .matched_items(0..matched)
            .map(|item| VisibleItem {
                session: item.data.session.clone(),
                text: item.data.text.clone(),
            })
            .collect();

        self.selected = selected_row(&self.visible_items, previous.as_ref(), self.selected);
    }

    /// Re-parse the current query string in the fuzzy matcher.
    fn reparse_query(&mut self, append: bool) {
        self.matcher.pattern.reparse(
            0,
            &self.query,
            CaseMatching::Smart,
            Normalization::Smart,
            append,
        );
    }

    /// Return the currently selected session, if any.
    fn selected_session(&self) -> Option<&Session> {
        self.visible_items
            .get(self.selected)
            .map(|item| &item.session)
    }

    /// Update the current repo context from the selected session.
    fn set_current_repo_from_selection(&mut self) {
        let Some(repo) = self.selected_session().and_then(Session::repo) else {
            return;
        };

        self.state.current_repo = Some(repo_context_path(repo));
    }

    /// Replace the current query string and update fuzzy matching state.
    fn set_query(&mut self, query: String) {
        let append = query.starts_with(&self.query);
        self.query = query;
        self.reparse_query(append);
    }
}

impl TerminalGuard {
    /// Switch the terminal into alternate-screen raw mode.
    fn new() -> anyhow::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen, cursor::Show);
        let _ = terminal::disable_raw_mode();
    }
}

/// Run the interactive picker for discovered sessions.
pub fn run(sessions: Vec<Session>, state: State) -> anyhow::Result<()> {
    let _guard = TerminalGuard::new()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new(sessions, state);

    loop {
        app.refresh_matches();
        terminal.draw(|frame| draw(frame, &mut app))?;

        if !event::poll(POLL_TIMEOUT)? {
            continue;
        }

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if handle_key(&mut app, key) {
                    break;
                }
            }
            Event::Resize(_, _) => {}
            _ => {}
        }
    }

    Ok(())
}

/// Render the picker UI for the current frame.
fn draw(frame: &mut ratatui::Frame<'_>, app: &mut App) {
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

    frame.render_widget(Paragraph::new(format!("Session: {}", app.query)), left[0]);
    frame.render_widget(
        Paragraph::new(format_status_line(
            left[1].width as usize,
            app.visible_items.len(),
            app.matcher.snapshot().item_count() as usize,
        )),
        left[1],
    );
    frame.render_widget(Paragraph::new(format!("  {}", app.state.header())), left[2]);

    let list_height = left[3].height as usize;
    app.scroll = scroll_offset(
        app.scroll,
        app.selected,
        app.visible_items.len(),
        list_height,
    );
    let lines = app
        .visible_items
        .iter()
        .enumerate()
        .skip(app.scroll)
        .take(list_height)
        .map(|(index, item)| {
            let prefix = if index == app.selected { "> " } else { "  " };
            Line::from(format!("{prefix}{}", item.text))
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Text::from(lines)), left[3]);

    let preview = app.preview_text(areas[1].width as usize);
    frame.render_widget(Paragraph::new(preview), areas[1]);
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

/// Handle a single keyboard event, returning `true` when the picker should exit.
fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Enter | KeyCode::Esc => true,
        KeyCode::Up => {
            app.move_up();
            false
        }
        KeyCode::Down => {
            app.move_down();
            false
        }
        KeyCode::Backspace => {
            app.pop_query();
            false
        }
        KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.move_down();
            false
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.move_up();
            false
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.set_current_repo_from_selection();
            false
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.clear_query();
            false
        }
        KeyCode::Char(c)
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT) =>
        {
            let mut query = app.query.clone();
            query.push(c);
            app.set_query(query);
            false
        }
        _ => false,
    }
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
fn selected_row(items: &[VisibleItem], previous: Option<&Session>, selected: usize) -> usize {
    if items.is_empty() {
        return 0;
    }

    previous
        .and_then(|previous| items.iter().position(|item| &item.session == previous))
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
            VisibleItem {
                session: Session::from_repo(PathBuf::from("/tmp/alpha")).unwrap(),
                text: "alpha".to_owned(),
            },
            VisibleItem {
                session: previous.clone(),
                text: "beta".to_owned(),
            },
        ];

        assert_eq!(selected_row(&items, Some(&previous), 0), 1);
    }

    #[test]
    fn renders_header_with_current_repo() {
        let state = State::new(Some(PathBuf::from("/tmp/repo")));

        assert_eq!(state.header(), "Current repo: /tmp/repo");
    }

    #[test]
    fn renders_header_without_current_repo() {
        let state = State::default();

        assert_eq!(state.header(), "Current repo: none");
    }

    #[test]
    fn strips_ansi_escape_sequences() {
        assert_eq!(strip_ansi("\u{1b}[31mhello\u{1b}[0m"), "hello");
    }
}
