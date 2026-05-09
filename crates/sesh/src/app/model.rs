// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Application model for discovered sessions and derived session candidates.

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use crate::jj;
use crate::session::Session;
use crate::tmux;

/// Application-level model for the session picker.
///
/// This owns the discovered session rows, the collision sets used while deriving candidate rows,
/// and the mapping from repository paths to workspace metadata needed by workspace-aware session
/// construction.
#[derive(Default)]
pub struct Model {
    sessions: Vec<Session>,
    seen_names: BTreeSet<String>,
    seen_repos: BTreeSet<PathBuf>,
}

impl Model {
    /// Discover live tmux sessions and repository-backed candidate sessions.
    pub async fn discover(repos: &[String]) -> anyhow::Result<Self> {
        let mut model = Self::default();

        // Add all the live sessions from tmux.
        for (name, info) in tmux::sessions().await? {
            model.seen_names.insert(name.clone());
            model.seen_repos.extend(info.repo.clone());
            model
                .sessions
                .push(Session::from_tmux(name, info.repo, info.alerts));
        }

        // Add an entry for every repo found, as long as it's not already associated with a
        // live tmux session.
        for repo in jj::repos(repos)? {
            let inserted = model.seen_repos.insert(repo.clone());
            if !inserted {
                continue;
            }

            let mut session = Session::from_repo(repo)?;

            // Make sure the name that will be used for a new session associated with this repo
            // will be unambiguous by adding a suffix.
            let mut i = 1;
            while !model.seen_names.insert(session.name()) {
                session.set_suffix(i.to_string());
                i += 1;
            }

            model.sessions.push(session);
        }

        Ok(model)
    }

    /// Construct the dynamic "new session" candidate for the current query and repo context.
    pub(super) fn new_session(&self, query: &str, repo: Option<&Path>) -> Option<Session> {
        let collides_with_tmux = self
            .sessions
            .iter()
            .any(|session| session.is_tmux() && session.name() == query);
        if query.is_empty() || collides_with_tmux {
            return None;
        }

        Some(Session::new(query.to_owned(), repo.map(Path::to_owned)))
    }

    /// Return the discovered sessions.
    pub(super) fn sessions(&self) -> &[Session] {
        &self.sessions
    }
}
