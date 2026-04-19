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

        jj::log(repo)
            .with_context(|| format!("failed to build preview for repo '{}'", repo.display()))
    }
}

impl<'a> From<&'a Session> for ListItem<'a> {
    fn from(session: &'a Session) -> Self {
        ListItem::new(session.text())
    }
}
