// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Session model and picker rendering.

use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use ratatui::widgets::ListItem;

use crate::cache::Preview;
use crate::jj;
use crate::path::TruncatedExt as _;
use crate::picker::Item;

/// A tmux session and optional repo metadata.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Session {
    name: String,
    repo: Option<PathBuf>,
}

impl Session {
    /// Construct a potential session from a repository path.
    ///
    /// The session's name is derived from the repository's root directory name.
    pub fn from_repo(path: PathBuf) -> anyhow::Result<Self> {
        let name = path
            .file_name()
            .context("invalid repo: no directory name")?
            .to_string_lossy()
            .into_owned();

        Ok(Self {
            name,
            repo: Some(path),
        })
    }

    /// Construct a potential session from information extracted from `tmux`.
    ///
    /// `name` is a tmux session name and `repo` is an optional path to a jj repository attached as
    /// a user-option on the tmux session.
    pub fn from_tmux(name: String, repo: Option<PathBuf>) -> Self {
        Self { name, repo }
    }

    /// Return the session name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the repository attached to this session, if any.
    pub fn repo(&self) -> Option<&Path> {
        self.repo.as_deref()
    }
}

impl Item for Session {
    fn text(&self) -> String {
        let Some(repo) = &self.repo else {
            return self.name().to_owned();
        };

        format!("{:<40} {}", self.name(), repo.truncated())
    }
}

impl Preview for Session {
    /// Render a `jj log` preview for this session's attached repository.
    fn preview(&self) -> anyhow::Result<String> {
        let Some(repo) = &self.repo else {
            return Ok(String::new());
        };

        let preview = jj::log(repo)
            .with_context(|| format!("failed to build preview for repo '{}'", repo.display()))?;

        Ok(strip_ansi(&preview))
    }
}

impl<'a> From<&'a Session> for ListItem<'a> {
    fn from(session: &'a Session) -> Self {
        ListItem::new(session.text())
    }
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
    use super::strip_ansi;

    #[test]
    fn strips_ansi_escape_sequences() {
        assert_eq!(strip_ansi("\u{1b}[31mhello\u{1b}[0m"), "hello");
    }
}
