// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Picker UI state, rendering, and input handling.

use std::fmt::Write as _;
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
use nucleo::Item;
use nucleo::Snapshot;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::widgets::HighlightSpacing;
use ratatui::widgets::List;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;

use crate::cache::PreviewCache;
use crate::path::TruncatedExt as _;
use crate::picker::Picker;
use crate::session::Session;
use crate::terminal::AlternateScreenGuard;
use crate::widget::loading::Loading;
use crate::widget::loading::LoadingState;

const POLL_TIMEOUT: Duration = Duration::from_millis(16);

/// Session picker state, caches, and UI behavior.
pub struct App {
    repo: Option<PathBuf>,
    picker: Picker<Session>,
    cache: PreviewCache<Session>,
    list: ListState,
    load: LoadingState,
}

impl App {
    /// Construct application state for the provided repo context.
    pub fn new(sessions: Vec<Session>, repo: Option<PathBuf>) -> Self {
        let picker = Picker::new(sessions.clone());
        let cache = PreviewCache::new(sessions);
        let list = ListState::default();
        let load = LoadingState::new();

        Self {
            repo,
            picker,
            cache,
            list,
            load,
        }
    }

    /// Run the interactive picker for discovered sessions.
    pub fn run(mut self) -> anyhow::Result<()> {
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

            if self.handle_key(key) {
                break;
            }
        }

        Ok(())
    }

    /// Draw the UI into the provided frame based on the current application state.
    ///
    /// The frame is split up into the following regions, each with its own widget:
    ///
    /// ```text
    /// +-----------------+-----------------------------+
    /// |> Prompt         | Preview                     |
    /// +-+---------------+ ...                         |
    /// |L| Header        |                             |
    /// +-+---------------+                             |
    /// | Session List    |                             |
    /// | ...             |                             |
    /// |                 |                             |
    /// |                 |                             |
    /// |                 |                             |
    /// +-----------------+-----------------------------+
    /// ```
    fn draw(&mut self, f: &mut ratatui::Frame<'_>) {
        use Constraint as C;
        use Direction as D;
        use Layout as L;

        // Split the frame into regions
        let cols = L::default()
            .direction(D::Horizontal)
            .constraints([C::Percentage(40), C::Percentage(60)])
            .split(f.area());

        let [sessions, preview] = &cols[..] else {
            panic!("expected two columns in the layout")
        };

        let rows = L::default()
            .direction(D::Vertical)
            .constraints([C::Length(1), C::Length(1), C::Min(0)])
            .split(*sessions);

        let [prompt, header, sessions] = &rows[..] else {
            panic!("expected three rows in the layout")
        };

        let cols = L::default()
            .direction(D::Horizontal)
            .constraints([C::Length(1), C::Min(0)])
            .split(*header);

        let [loading, header] = &cols[..] else {
            panic!("expected two columns in header");
        };

        // Poll the picker for its latest state, and build the data model.
        let (status, snapshot, query) = self.picker.refresh();
        let items: Vec<_> = snapshot.matched_items(..).collect();
        let selected = self.list.selected().and_then(|s| {
            if s < items.len() {
                items.get(s)
            } else {
                items.last()
            }
        });

        f.render_widget(prompt_widget(query), *prompt);
        f.render_stateful_widget(Loading(status.running), *loading, &mut self.load);
        f.render_widget(header_widget(snapshot, self.repo.as_deref()), *header);
        f.render_stateful_widget(session_list_widget(snapshot), *sessions, &mut self.list);
        f.render_widget(preview_widget(&self.cache, selected), *preview);
    }

    /// Handle a single keyboard event, returning `true` when the picker should exit.
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        use KeyCode as KC;
        use KeyModifiers as KM;
        const CTRL: KM = KM::CONTROL;

        match key.code {
            KC::Enter => return true,

            KC::Esc => return true,
            KC::Char('g' | 'c') if key.modifiers.contains(CTRL) => return true,

            KC::Up => self.list.select_previous(),
            KC::Char('p') if key.modifiers.contains(CTRL) => self.list.select_previous(),

            KC::Down => self.list.select_next(),
            KC::Char('n') if key.modifiers.contains(CTRL) => self.list.select_next(),

            KC::Backspace => self.picker.pop(),
            KC::Char('u') if key.modifiers.contains(CTRL) => self.picker.clear(),

            KC::Char('r') if key.modifiers.contains(CTRL) => self.set_current_repo(),

            KC::Char(c) if key.modifiers.is_empty() => self.picker.push(c),
            _ => {}
        };

        false
    }

    /// Set the current repo from the currently selected session.
    ///
    /// If there is no selection, or the selected session has no associated repo, the current repo
    /// is cleared.
    fn set_current_repo(&mut self) {
        let mut items = self.picker.snapshot().matched_items(..).map(|i| i.data);
        let selected = self.list.selected().and_then(|s| {
            if s < items.len() {
                items.nth(s)
            } else {
                items.next_back()
            }
        });

        self.repo = selected.and_then(|s| s.repo()).map(|p| p.to_owned());
    }
}

/// Build the prompt widget for the active query string.
fn prompt_widget(query: &str) -> impl Widget {
    Paragraph::new(format!("> {query}"))
}

/// Build the header widget with match counts and current repo context.
fn header_widget(snapshot: &Snapshot<Session>, repo: Option<&Path>) -> impl Widget {
    let found = snapshot.matched_items(..).count();
    let total = snapshot.item_count();
    let width = if total == 0 {
        1
    } else {
        total.ilog10() as usize + 1
    };

    let mut line = format!(" {found:>width$}/{total} | [C-r] repo: ");
    if let Some(repo) = repo {
        write!(line, "{}", repo.truncated()).unwrap();
    } else {
        line.push_str("none");
    }

    Paragraph::new(line)
}

/// Build the session list widget for the current fuzzy-match snapshot.
fn session_list_widget(snapshot: &Snapshot<Session>) -> impl StatefulWidget<State = ListState> {
    List::new(snapshot.matched_items(..).map(|i| i.data))
        .highlight_symbol("> ")
        .highlight_spacing(HighlightSpacing::Always)
}

/// Build the preview widget for the currently selected session.
fn preview_widget(
    cache: &PreviewCache<Session>,
    selected: Option<&Item<'_, Session>>,
) -> impl Widget {
    let Some(session) = selected else {
        return Paragraph::new("");
    };

    let Some(preview) = cache.get(&session.matcher_columns[0]) else {
        return Paragraph::new("Loading...");
    };

    match preview.as_ref() {
        Ok(preview) => Paragraph::new(preview.clone()),
        Err(err) => Paragraph::new(format!("Error: {err}")),
    }
}
