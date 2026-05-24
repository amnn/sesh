// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Application model for discovered sessions and derived session candidates.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use futures::stream::FuturesUnordered;
use futures::stream::StreamExt as _;
use nucleo::Snapshot;
use nucleo::Status;

use crate::jj;
use crate::picker::Picker;
use crate::session::Base;
use crate::session::LiveKind;
use crate::session::NewKind;
use crate::session::RepoKind;
use crate::session::Session;
use crate::tmux;

/// Application-level model for the session picker.
///
/// This owns the discovered session rows, the collision sets used while deriving candidate rows,
/// and the mapping from repository paths to workspace metadata needed by workspace-aware session
/// construction.
pub struct Model {
    /// Fuzzy finder state for discovered sessions.
    picker: Picker<Session>,

    /// The sessions to fuzzy find over.
    sessions: Vec<Session>,

    /// Names of live tmux sessions, used to disambiguate candidate session names.
    seen_tmux_names: BTreeSet<String>,

    /// Workspaces found, identified by their root (default) path and workspace name. Used to
    /// disambiguate the creation of new workspaces.
    seen_workspaces: BTreeMap<PathBuf, BTreeSet<String>>,

    /// Mapping from a repository path to optional workspace metadata.
    ///
    /// A present `None` value means workspace discovery succeeded for this path, but no workspace
    /// root was recorded for it.
    workspaces: BTreeMap<PathBuf, Option<Workspace>>,
}

/// Workspace metadata for a discovered repository.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Workspace {
    /// Workspace name reported by `jj workspace list` for this repository root.
    ///
    /// `None` represents the default workspace.
    name: Option<String>,
    /// Root path for the default workspace in the same jj repository, when available.
    default: Option<PathBuf>,
}

impl Model {
    /// Construct a model with discovered sessions and a seeded fuzzy query.
    ///
    /// `globs` is a list of glob patterns to search for repositories in. `current` is an
    /// optional current repository path. The model will discover workspace information for all the
    /// repositories found between the two. `query` seeds the model's fuzzy picker.
    pub async fn new(
        globs: &[String],
        current: Option<&Path>,
        query: String,
    ) -> anyhow::Result<Self> {
        let mut model = Self {
            picker: Picker::new(query),
            sessions: Vec::new(),
            seen_tmux_names: BTreeSet::new(),
            seen_workspaces: BTreeMap::new(),
            workspaces: BTreeMap::new(),
        };

        model.discover(globs, current).await?;
        Ok(model)
    }

    /// Return all matched sessions after the matcher has finished processing pending updates.
    pub fn matches(&mut self) -> Vec<Session> {
        loop {
            let (status, snapshot, _) = self.picker.refresh();
            if !status.running {
                return snapshot
                    .matched_items(..)
                    .map(|item| item.data.clone())
                    .collect();
            }
        }
    }

    /// Clear the active query string.
    pub(crate) fn clear_query(&mut self) {
        self.picker.clear();
    }

    /// Discover sessions while preserving the current fuzzy query.
    pub(crate) async fn discover(
        &mut self,
        globs: &[String],
        current: Option<&Path>,
    ) -> anyhow::Result<()> {
        self.sessions.clear();
        self.seen_tmux_names.clear();
        self.seen_workspaces.clear();
        self.workspaces.clear();

        let mut tmux_repos = BTreeSet::new();

        let tmux_sessions = tmux::sessions().await?;
        for (name, info) in &tmux_sessions {
            self.seen_tmux_names.insert(name.clone());
            tmux_repos.extend(info.repo.clone());
        }

        let globbed = jj::repos(globs)?;
        let repos = globbed
            .iter()
            .chain(&tmux_repos)
            .map(PathBuf::as_path)
            .chain(current);

        // Discover workspace names and locations for the repositories found -- this is used to
        // construct workspace-aware session names.
        self.workspaces = workspaces(repos).await;

        // Add all the live sessions from tmux.
        for (name, info) in tmux_sessions {
            let can_delete = info
                .repo
                .as_ref()
                .and_then(|repo| self.workspace_name(repo))
                .is_some();

            let session = LiveKind::new(name, info.repo, info.alerts, can_delete);
            self.sessions.push(session.into());
        }

        // Add an entry for every repo found, as long as it's not already associated with a
        // live tmux session.
        for repo in globbed {
            if tmux_repos.contains(&repo) {
                continue;
            }

            let mut session = if let Some(Some(workspace)) = self.workspaces.get(&repo) {
                RepoKind::new(
                    workspace.name.as_deref(),
                    workspace.default.clone().unwrap_or_else(|| repo.to_owned()),
                    repo.to_owned(),
                    workspace.name.is_some(),
                )
            } else {
                RepoKind::new(None, repo.to_owned(), repo.to_owned(), false)
            };

            session.disambiguate(&self.seen_tmux_names);
            self.sessions.push(session.into());
        }

        // Attach information about workspaces seen, to help disambiguate future new workspaces
        // created by the tool.
        for (root, workspace) in &self.workspaces {
            let Some(workspace) = workspace else {
                continue;
            };

            let root = workspace.default.as_ref().unwrap_or(root);
            let name = workspace.name.as_deref().unwrap_or(jj::DEFAULT_WORKSPACE);

            self.seen_workspaces
                .entry(root.to_owned())
                .or_default()
                .insert(name.to_owned());
        }

        self.picker.reset();
        self.picker.inject(self.sessions.clone());

        Ok(())
    }

    /// Construct the dynamic "new session" candidate for the current query and repo context.
    pub(crate) fn new_session(&self, repo: Option<&Path>) -> Option<Session> {
        let query = self.picker.query();
        if query.is_empty() {
            return None;
        }

        let base = match repo {
            None => Base::Cwd(None),
            Some(repo) => match self.workspaces.get(repo) {
                None => Base::Cwd(Some(repo.to_owned())),
                Some(workspace) => Base::Repo {
                    default: workspace
                        .as_ref()
                        .and_then(|w| w.default.clone())
                        .unwrap_or_else(|| repo.to_owned()),
                    revision: jj::DEFAULT_BASE_REVSET.to_owned(),
                },
            },
        };

        let empty = BTreeSet::new();
        let siblings = match &base {
            Base::Repo { default, .. } => self.seen_workspaces.get(default).unwrap_or(&empty),
            Base::Cwd(_) => &empty,
        };

        let mut session = NewKind::new(query, base);
        session.disambiguate(&self.seen_tmux_names, siblings);
        Some(session.into())
    }

    /// Remove the trailing character from the active query string.
    pub(crate) fn pop_query(&mut self) {
        self.picker.pop();
    }

    /// Append one character to the active query string.
    pub(crate) fn push_query(&mut self, ch: char) {
        self.picker.push(ch);
    }

    /// Refresh fuzzy matches and return the currently visible rows.
    pub(crate) fn refresh(&mut self) -> (Status, &Snapshot<Session>, &str) {
        self.picker.refresh()
    }

    /// Return the discovered sessions.
    pub(crate) fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    /// Return the exact jj workspace name for `repo`, if it is a named workspace.
    pub(crate) fn workspace_name(&self, repo: &Path) -> Option<&str> {
        self.workspaces
            .get(repo)
            .and_then(|w| w.as_ref())
            .and_then(|w| w.name.as_deref())
    }
}

/// Discover workspace metadata for every workspace associated with each repository.
async fn workspaces<'a>(
    repos: impl IntoIterator<Item = &'a Path>,
) -> BTreeMap<PathBuf, Option<Workspace>> {
    let mut tasks: FuturesUnordered<_> = repos
        .into_iter()
        .map(|repo| async move { (repo.to_owned(), jj::workspaces(repo).await) })
        .collect();

    let mut info = BTreeMap::new();
    while let Some((repo, Ok(ws))) = tasks.next().await {
        let default = ws.get(&None).cloned().flatten();
        let mut found = false;
        for (name, root) in ws {
            let Some(root) = root else {
                continue;
            };

            found |= root == repo;
            let default = default.clone();
            info.insert(root, Some(Workspace { name, default }));
        }

        if !found {
            info.insert(repo, None);
        }
    }

    info
}
