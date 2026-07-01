// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Picker UI state, rendering, and input handling.

mod component;
mod header;
mod highlight;
mod layout;
mod onto;
mod sessions;
mod span;

use std::io;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context as _;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::component::block::Block;
use crate::app::component::prompt;
use crate::app::component::spinner;
use crate::app::component::spinner::Spinner;
use crate::app::header::Header;
use crate::app::sessions::Sessions;
use crate::app::sessions::preview;
use crate::app::sessions::preview::Preview;
use crate::cmd::jj;
use crate::model::Model;
use crate::model::session::Repo;
use crate::model::session::Session;
use crate::terminal::AlternateScreenGuard;

/// Timeout for waiting for a key event.
const POLL_TIMEOUT: Duration = Duration::from_millis(16);

/// Session picker state, caches, and UI behavior.
pub struct App {
    onto: Option<onto::State>,
    repo: Option<Repo>,
    spinner: spinner::State,
    model: Model,
    preview: preview::State,
    sessions: sessions::State,
}

/// Runtime inputs used by the interactive picker but not owned by its UI state.
pub struct Context<'a> {
    /// Repository globs to discover alongside existing tmux sessions.
    pub globs: &'a [String],

    /// Shell setup to run when creating a tmux session.
    pub setup: &'a str,

    /// Character used to mark live tmux sessions in the picker.
    pub sigil: char,
}

/// Completed action chosen from the picker.
enum Action {
    /// Do nothing and exit the picker.
    Cancel,

    /// Close the selected tmux session without deleting any attached workspace.
    Close(Session),

    /// Delete the selected session's attached workspace checkout, closing tmux if live.
    Delete(Session),

    /// Create the selected session without switching to it.
    Create(Session),

    /// Switch to the selected session, creating it first if needed.
    Switch(Session),

    /// Toggle the selected live session's manual flag.
    ToggleFlag(Session),
}

impl App {
    /// Create a new application.
    ///
    /// `repo` is the initial base repository. `model` contains the underlying data to drive the
    /// interface.
    pub fn new(repo: Option<PathBuf>, model: Model) -> Self {
        let mut preview = preview::State::new();
        preview.feed(model.sessions());

        Self {
            onto: None,
            repo: repo.map(Repo::new),
            spinner: spinner::State::new(),
            model,
            preview,
            sessions: sessions::State::new(),
        }
    }

    /// Run the interactive picker for discovered sessions.
    pub async fn run(mut self, cwd: &Path, ctx: Context<'_>) -> anyhow::Result<()> {
        let guard = AlternateScreenGuard::new()?;
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

        loop {
            loop {
                terminal.draw(|frame| self.draw(frame, ctx.sigil))?;

                if !event::poll(POLL_TIMEOUT)? {
                    continue;
                }

                let Event::Key(key) = event::read()? else {
                    continue;
                };

                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match self.handle_key(key) {
                    None => continue,
                    Some(Action::Cancel) => return Ok(()),

                    Some(Action::Close(session)) => {
                        session.close().await?;
                        break;
                    }

                    Some(Action::Delete(session)) => {
                        self.delete(&session).await?;
                        session.close().await?;
                        self.sessions.reset_delete();
                        break;
                    }

                    Some(Action::Create(session)) => {
                        session.create(cwd, ctx.setup).await?;
                        self.sessions.select_first();
                        self.model.clear_query();
                        break;
                    }

                    Some(Action::Switch(session)) => {
                        drop(guard);
                        session.switch(cwd, ctx.setup).await?;
                        return Ok(());
                    }

                    Some(Action::ToggleFlag(session)) => {
                        session.toggle_flag().await?;
                        break;
                    }
                }
            }

            self.discover(ctx.globs).await?;
        }
    }

    /// Delete the repository or workspace checkout attached to `session`, if any.
    async fn delete(&self, session: &Session) -> anyhow::Result<()> {
        let Some(repo) = session.repo() else {
            return Ok(());
        };

        if let Some(name) = self.model.workspace_name(&repo) {
            jj::forget_workspace(&repo, name).await?;
        }

        match tokio::fs::remove_dir_all(&repo).await {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err)
                .with_context(|| format!("failed to remove repository '{}'", repo.display())),
        }
    }

    /// Discover sessions and inject them into the picker.
    async fn discover(&mut self, globs: &[String]) -> anyhow::Result<()> {
        let repo = self.repo.as_ref().map(|r| r.source());
        self.model.discover(globs, repo).await?;
        self.preview.feed(self.model.sessions());
        Ok(())
    }

    /// Draw the UI into the provided frame based on the current application state.
    ///
    /// The frame is split up into regions, each with its own widget. The `preview` region and its
    /// scroll bar are only visible when the preview is toggled on (defaults to visible).
    fn draw(&mut self, f: &mut ratatui::Frame<'_>, sigil: char) {
        let l = layout::Layout::new(f.area(), self.preview.visible() || self.onto.is_some());

        let new_session = self.model.new_session(self.repo.as_ref());

        // Poll the picker for its latest state, and build the data model.
        let (status, snapshot, query) = self.model.refresh();
        let items: Vec<_> = snapshot.matched_items(..).collect();

        let (label, query) = if let Some(onto) = &self.onto {
            ("onto", onto.query())
        } else {
            ("session", query)
        };

        // (1) Render picker state
        f.render_widget(prompt::widget(label, query), l.prompt);
        f.render_stateful_widget(Spinner::new(status.running), l.loading, &mut self.spinner);

        let sessions = Sessions::new(
            sigil,
            new_session,
            &items,
            snapshot.pattern().column_pattern(0),
        );

        // (2) Render session list. This also updates `self.sessions`, so that the selected index
        // and session are up-to-date and valid.
        sessions.draw(f, l.sessions, l.scroll, &mut self.sessions);

        // (2.a) Ensure the currently selected session is fed into the preview cache. Most sessions
        // have already been fed to preview during discovery and this will do nothing, but if the
        // selected row corresponds to the new session, then its repo may not have been fed to
        // preview yet.
        self.preview.feed(self.sessions.selected());

        let header = Header::new(
            self.sessions.is_deleting(),
            items.len(),
            self.repo.as_ref(),
            self.sessions.selected(),
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

        // (4) Render the selected session preview or current-repo onto-picker surface, if it is
        // toggled on.
        if let Some(onto) = &mut self.onto {
            onto.draw(f, l_preview);
        } else {
            let preview = Preview::new(self.sessions.preview());
            preview.draw(f, l_preview, &mut self.preview);
        }
    }

    /// Handle a single keyboard event, returning the consequent application action.
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        use KeyCode as KC;
        use KeyModifiers as KM;

        const ALT: KM = KM::ALT;
        const CTRL: KM = KM::CONTROL;
        const SHIFT: KM = KM::SHIFT;

        if self.sessions.is_deleting() {
            self.sessions.reset_delete();

            match key.code {
                KC::Char('y') if key.modifiers.contains(CTRL) => {
                    return self.sessions.take_selected().map(Action::Delete);
                }

                KC::Esc => return None,
                KC::Char('c') if key.modifiers.contains(CTRL) => return None,

                _ => {}
            }
        }

        if let Some(onto) = &mut self.onto {
            match onto.handle_key(key) {
                Some(onto::Action::Cancel) => self.onto = None,
                None => {}
            }

            return None;
        }

        match key.code {
            // Accept the selected row.
            KC::Enter => return self.sessions.take_selected().map(Action::Switch),

            // Create the selected row without switching.
            KC::Char('n') if key.modifiers.contains(CTRL) => {
                return self.sessions.take_selected().map(Action::Create);
            }

            // Cancel
            KC::Esc => return Some(Action::Cancel),
            KC::Char('c' | 'g') if key.modifiers.contains(CTRL) => return Some(Action::Cancel),

            // Session actions
            KC::Char('x') if key.modifiers.contains(CTRL) && self.sessions.can_close() => {
                return self.sessions.take_selected().map(Action::Close);
            }

            KC::Char('d') if key.modifiers.contains(CTRL) && self.sessions.can_delete() => {
                self.sessions.start_delete();
            }

            KC::Char('f') if key.modifiers.contains(CTRL) && self.sessions.can_flag() => {
                return self.sessions.take_selected().map(Action::ToggleFlag);
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
            KC::Char('o') if key.modifiers.contains(CTRL) => {
                if let Some(repo) = &self.repo {
                    self.onto = Some(onto::State::new(repo.source().to_owned()));
                }
            }

            KC::Char('r') if key.modifiers.contains(ALT) => self.reset_current_repo(),
            KC::Char('r') if key.modifiers.contains(CTRL) => self.set_current_repo(),

            // View state
            KC::Char('p') if key.modifiers.contains(CTRL) => {
                self.preview.toggle();
            }

            // Edit query
            KC::Backspace => self.model.pop_query(),
            KC::Char('u') if key.modifiers.contains(CTRL) => self.model.clear_query(),
            KC::Char(c) if key.modifiers.is_empty() => self.model.push_query(c),
            KC::Char(c) if key.modifiers.contains(SHIFT) => self.model.push_query(c),

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
            .and_then(Session::repo)
            .map(Repo::new);
    }
}
