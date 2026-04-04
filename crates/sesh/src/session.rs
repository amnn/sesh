// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Session model and picker rendering.

use std::path::PathBuf;

use anyhow::Context as _;

use crate::jj;
use crate::path::TruncatedExt as _;

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
    /// `name` is a tmux session name and `repo` is an optional path to a jj repository attached
    /// as a user-option on the tmux session.
    pub fn from_tmux(name: String, repo: Option<PathBuf>) -> Self {
        Self { name, repo }
    }

    /// Return the text shown for this session in the picker list.
    pub fn item(&self) -> String {
        let Some(repo) = &self.repo else {
            return self.name().to_owned();
        };

        format!("{:<40} {}", self.name(), repo.truncated())
    }

    /// Return the session name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Render a `jj log` preview for this session's attached repository.
    pub fn preview(&self, _width: usize) -> anyhow::Result<String> {
        let Some(repo) = &self.repo else {
            return Ok(String::new());
        };

        jj::log(repo)
            .with_context(|| format!("failed to build preview for repo '{}'", repo.display()))
    }

    /// Return the repository attached to this session, if any.
    pub fn repo(&self) -> Option<&std::path::Path> {
        self.repo.as_deref()
    }
}
