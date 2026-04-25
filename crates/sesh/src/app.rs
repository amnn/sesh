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
use nucleo::Item;
use nucleo::Snapshot;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets::HighlightSpacing;
use ratatui::widgets::List;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;

use crate::cache::PreviewCache;
use crate::picker::Picker;
use crate::session::Session;
use crate::terminal::AlternateScreenGuard;
use crate::ui::push_repo_path_spans;
use crate::ui::push_shortcut_span;
use crate::widget::Loading;
use crate::widget::LoadingState;

const POLL_TIMEOUT: Duration = Duration::from_millis(16);

/// Session picker state, caches, and UI behavior.
pub struct App {
    repo: Option<PathBuf>,
    picker: Picker<Session>,
    cache: PreviewCache<Session>,
    list: ListState,
    load: LoadingState,
    preview_scroll: usize,
}

impl App {
    /// Construct application state for the provided repo context.
    pub fn new(sessions: Vec<Session>, repo: Option<PathBuf>) -> Self {
        let picker = Picker::new(sessions.clone());
        let cache = PreviewCache::new(sessions);
        let list = ListState::default();
        let load = LoadingState::new();
        let preview_scroll = 0;

        Self {
            repo,
            picker,
            cache,
            list,
            load,
            preview_scroll,
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
    /// +-----------------+-+-------------------------+-+
    /// |> Prompt         |S| Preview                 |S|
    /// +-+---------------+c| ...                     |c|
    /// |L| Header        |r|                         |r|
    /// +-+---------------+o|                         |o|
    /// | Session List    |l|                         |l|
    /// | ...             |l|                         |l|
    /// |                 | |                         | |
    /// |                 | |                         | |
    /// |                 | |                         | |
    /// +-----------------+-+-------------------------+-+
    /// ```
    fn draw(&mut self, f: &mut ratatui::Frame<'_>) {
        use Constraint as C;
        use Direction as D;
        use Layout as L;

        // Split the frame into regions
        let cols = L::default()
            .direction(D::Horizontal)
            .constraints([C::Percentage(40), C::Length(1), C::Percentage(60)])
            .split(f.area());

        let [sessions, scroll, preview] = &cols[..] else {
            panic!("expected three columns in the layout")
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

        // If the list does not have a selection, set it to the first visible item.
        if self.list.selected().is_none() && !items.is_empty() {
            let first = self.list.offset().min(items.len() - 1);
            self.list.select(Some(first));
        }

        // Render the header and session list.
        f.render_widget(prompt_widget(query), *prompt);
        f.render_stateful_widget(Loading(status.running), *loading, &mut self.load);
        f.render_widget(header_widget(snapshot, self.repo.as_deref()), *header);
        f.render_stateful_widget(session_list_widget(snapshot), *sessions, &mut self.list);

        // Rendering corrects the list's selected index, so we find the selected item.
        let selected = self.list.selected().and_then(|s| items.get(s));
        let text = preview_widget(&self.cache, selected);

        // Sync scroll states
        let mut session_scroll = ScrollbarState::default()
            .content_length(items.len().saturating_sub(sessions.height as usize).max(1))
            .viewport_content_length(sessions.height as usize)
            .position(self.list.offset());

        self.preview_scroll = self
            .preview_scroll
            .clamp(0, text.height().saturating_sub(preview.height as usize));

        let preview_content_length = text
            .height()
            .checked_sub(preview.height as usize + 1)
            .map_or(0, |n| n + 2);

        let mut preview_scroll = ScrollbarState::default()
            .content_length(preview_content_length)
            .viewport_content_length(preview.height as usize)
            .position(self.preview_scroll);

        f.render_stateful_widget(scrollbar_widget(), *scroll, &mut session_scroll);
        f.render_stateful_widget(scrollbar_widget(), *preview, &mut preview_scroll);

        let preview_para = Paragraph::new(text).scroll((self.preview_scroll as u16, 0));
        f.render_widget(preview_para, *preview);
    }

    /// Handle a single keyboard event, returning `true` when the picker should exit.
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        use KeyCode as KC;
        use KeyModifiers as KM;
        const CTRL: KM = KM::CONTROL;

        match key.code {
            // Quit successfully
            KC::Enter => return true,

            // Cancel
            KC::Esc => return true,
            KC::Char('g' | 'c') if key.modifiers.contains(CTRL) => return true,

            // Scroll preview
            KC::Char('p') if key.modifiers.contains(KM::ALT) => {
                self.preview_scroll = self.preview_scroll.saturating_sub(1);
            }

            KC::Up if key.modifiers.contains(KM::SHIFT) => {
                self.preview_scroll = self.preview_scroll.saturating_sub(1);
            }

            KC::Char('n') if key.modifiers.contains(KM::ALT) => {
                self.preview_scroll = self.preview_scroll.saturating_add(1);
            }

            KC::Down if key.modifiers.contains(KM::SHIFT) => {
                self.preview_scroll = self.preview_scroll.saturating_add(1);
            }

            // Session list selection
            KC::Up => self.list.select_previous(),
            KC::Down => self.list.select_next(),

            // Edit query
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

/// Build the header widget with match counts and current repo context.
fn header_widget(snapshot: &Snapshot<Session>, repo: Option<&Path>) -> impl Widget {
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

    line
}

fn preview_widget(
    cache: &PreviewCache<Session>,
    selected: Option<&Item<'_, Session>>,
) -> Text<'static> {
    let Some(session) = selected else {
        return Text::from("");
    };

    let Some(preview) = cache.get(&session.matcher_columns[0]) else {
        return Text::from("Loading...");
    };

    match preview.as_ref() {
        Ok(preview) => preview.clone(),
        Err(err) => Text::from(format!("Error: {err}")),
    }
}

/// Build the prompt widget for the active query string.
fn prompt_widget(query: &str) -> impl Widget {
    Line::from(vec![
        Span::styled("session: ", Style::new().dim()),
        Span::raw(query.to_owned()),
    ])
}

/// Build the scrollbar widget that visually separates the session list from the
/// preview.
fn scrollbar_widget() -> impl StatefulWidget<State = ScrollbarState> {
    Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .track_symbol(Some("│"))
        .thumb_symbol("┃")
}

/// Build the session list widget for the current fuzzy-match snapshot.
fn session_list_widget(snapshot: &Snapshot<Session>) -> impl StatefulWidget<State = ListState> {
    List::new(snapshot.matched_items(..).map(|i| i.data))
        .highlight_style(Style::new().reversed())
        .highlight_symbol(Span::styled("▌", Style::new().bg(Color::Red)))
        .highlight_spacing(HighlightSpacing::Always)
}
