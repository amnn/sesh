// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Session model and skim item rendering.

use std::borrow::Cow;
use std::path::PathBuf;

use anyhow::Context as _;
use skim::prelude::ItemPreview;
use skim::prelude::PreviewContext;
use skim::prelude::SkimItem;

use crate::jj;
use crate::path::TruncatedExt as _;

/// A tmux session and optional repo metadata.
#[derive(Clone, Debug)]
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
    /// `name` is a tmux session name and `repo` is an
    /// optional path to a jj repository, that is attached as a user-option on the tmux session.
    pub fn from_tmux(name: String, repo: Option<PathBuf>) -> Self {
        Self { name, repo }
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
}

impl SkimItem for Session {
    fn preview(&self, context: PreviewContext) -> ItemPreview {
        match self.preview(context.width) {
            Ok(preview) => ItemPreview::Text(preview),
            Err(error) => ItemPreview::Text(format!("Failed to render preview: {error:?}")),
        }
    }

    fn text(&self) -> Cow<'_, str> {
        let Some(repo) = &self.repo else {
            return self.name().into();
        };

        format!("{:<40} {}", self.name(), repo.truncated()).into()
    }
}
