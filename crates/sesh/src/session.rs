// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Session model and picker rendering.

use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use async_trait::async_trait;
use ratatui::style::Style;
use ratatui::style::Stylize as _;
use ratatui::text::Line;
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr as _;

use crate::app::row::Row;
use crate::cache::Preview;
use crate::jj;
use crate::path::TruncatedExt as _;
use crate::picker::Item;
use crate::tmux;
use crate::ui::Highlight;
use crate::ui::push_repo_path_spans;

const NAME_WIDTH: usize = 40;
const SIGIL_TMUX: &str = "⬤";
const SUFFIX_DELIM: &str = "~";

/// A tmux session and optional repo metadata.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Session {
    name: String,
    repo: Option<PathBuf>,
    suffix: Option<String>,
    alerts: Vec<String>,
    tmux: bool,
}

impl Session {
    /// Construct a potential session from a repository path and session name.
    pub fn from_repo(name: String, path: PathBuf) -> Self {
        Self {
            name,
            repo: Some(path),
            suffix: None,
            alerts: vec![],
            tmux: false,
        }
    }

    /// Construct a potential session from information extracted from `tmux`.
    ///
    /// `name` is a tmux session name, `repo` is an optional path to a jj repository attached as a
    /// user-option on the tmux session, and `alerts` is a list of windows in the session that have
    /// an active bell alert.
    pub fn from_tmux(name: String, repo: Option<PathBuf>, alerts: Vec<String>) -> Self {
        if let Some((name, suffix)) = name.rsplit_once(SUFFIX_DELIM) {
            Self {
                alerts,
                name: name.to_owned(),
                repo,
                suffix: Some(suffix.to_owned()),
                tmux: true,
            }
        } else {
            Self {
                name,
                repo,
                suffix: None,
                alerts,
                tmux: true,
            }
        }
    }

    /// Construct a potential session from a name and optional repository path.
    pub fn new(name: String, repo: Option<PathBuf>) -> Self {
        Self {
            alerts: vec![],
            name,
            repo,
            suffix: None,
            tmux: false,
        }
    }

    /// Return whether this entry represents a currently live tmux session.
    pub fn is_tmux(&self) -> bool {
        self.tmux
    }

    /// Return the session name.
    pub fn name(&self) -> String {
        if let Some(suffix) = &self.suffix {
            self.name.clone() + SUFFIX_DELIM + suffix
        } else {
            self.name.clone()
        }
    }

    /// Return the repository attached to this session, if any.
    pub fn repo(&self) -> Option<&Path> {
        self.repo.as_deref()
    }

    /// Update the suffix used to disambiguate this session's name from all the others.
    pub fn set_suffix(&mut self, suffix: String) {
        self.suffix = Some(suffix);
    }

    /// Switch the current tmux client to this session, creating the session first if needed.
    pub async fn switch(&self, cwd: &Path, setup: &str) -> anyhow::Result<()> {
        self.ensure_tmux(cwd, setup).await?;
        tmux::switch_client(&self.switch_target()).await
    }

    /// Ensure the tmux session we are switching to is ready.
    async fn ensure_tmux(&self, cwd: &Path, setup: &str) -> anyhow::Result<()> {
        if self.tmux {
            return Ok(());
        }

        let target = self.name();
        let cwd = self.repo().unwrap_or(cwd);
        tmux::new_session(&target, cwd).await?;

        if let Some(repo) = self.repo() {
            tmux::set_option(&target, "@sesh.repo", repo).await?;
        }

        tmux::run_shell(&format!("{target}:0"), cwd, setup).await?;

        Ok(())
    }

    /// Return the tmux target for switching to this session.
    fn switch_target(&self) -> String {
        let session = self.name();
        if let Some(window) = self.alerts.first() {
            format!("{session}:{window}")
        } else {
            session
        }
    }
}

impl Item for Session {
    type Widget = Row;

    fn render(&self, highlighted: bool, matches: &[u32]) -> Self::Widget {
        let mut hl = Highlight::new(matches.to_vec());
        let mut line = Line::default();
        push_session_name_spans(&mut line, self, &mut hl);

        if let Some(repo) = &self.repo {
            let padding = NAME_WIDTH.saturating_sub(self.name().width()) + 1;
            line.extend(hl.highlight(Span::raw(" ".repeat(padding))));
            push_repo_path_spans(&mut line, repo, &mut hl);
        };

        let row = Row::new(line);
        let alert = !self.alerts.is_empty();
        if !self.tmux {
            row
        } else if alert && highlighted {
            row.with_sigil(Span::raw(SIGIL_TMUX).on_green())
        } else if alert {
            row.with_sigil(Span::raw(SIGIL_TMUX).green())
        } else {
            row.with_sigil(Span::raw(SIGIL_TMUX).dim())
        }
    }

    fn text(&self) -> String {
        let Some(repo) = &self.repo else {
            return self.name();
        };

        format!(
            "{:<NAME_WIDTH$} {}",
            self.name(),
            repo.truncated().display()
        )
    }
}

#[async_trait]
impl Preview for Session {
    /// Render a `jj log` preview for this session's attached repository.
    async fn preview(&self) -> anyhow::Result<String> {
        let Some(repo) = &self.repo else {
            return Ok(String::new());
        };

        jj::log(repo)
            .await
            .with_context(|| format!("failed to build preview for repo '{}'", repo.display()))
    }
}

fn push_session_name_spans<'a>(
    spans: &mut impl Extend<Span<'a>>,
    session: &Session,
    hl: &mut Highlight,
) {
    spans.extend(hl.highlight(Span::raw(session.name.clone())));

    if let Some(suffix) = &session.suffix {
        spans.extend(hl.highlight(Span::styled(
            format!("{SUFFIX_DELIM}{suffix}"),
            Style::new().dim(),
        )));
    }
}
