// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Application model for discovered sessions and derived session candidates.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use futures::stream::FuturesUnordered;
use futures::stream::StreamExt as _;

use crate::jj;
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
#[derive(Default)]
pub struct Model {
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
    /// Discover live tmux sessions and repository-backed candidate sessions.
    ///
    /// `globs` is a list of glob patterns to search for repositories in. `current` is an
    /// optional current repository path. The model will discover workspace information for all the
    /// repositories found between the two.
    pub async fn discover(globs: &[String], current: Option<&Path>) -> anyhow::Result<Self> {
        let mut model = Self::default();
        let mut tmux_repos = BTreeSet::new();

        // Add all the live sessions from tmux.
        for (name, info) in tmux::sessions().await? {
            model.seen_tmux_names.insert(name.clone());
            tmux_repos.extend(info.repo.clone());

            let session = LiveKind::new(name, info.repo, info.alerts);
            model.sessions.push(session.into());
        }

        let globbed = jj::repos(globs)?;
        let repos = globbed
            .iter()
            .chain(&tmux_repos)
            .map(PathBuf::as_path)
            .chain(current);

        // Discover workspace names and locations for the repositories found -- this is used to
        // construct workspace-aware session names.
        model.workspaces = workspaces(repos).await;

        // Add an entry for every repo found, as long as it's not already associated with a
        // live tmux session.
        for repo in globbed {
            if tmux_repos.contains(&repo) {
                continue;
            }

            let mut session = if let Some(Some(workspace)) = model.workspaces.get(&repo) {
                RepoKind::new(
                    workspace.name.as_deref(),
                    workspace.default.clone().unwrap_or_else(|| repo.to_owned()),
                    repo.to_owned(),
                )
            } else {
                RepoKind::new(None, repo.to_owned(), repo.to_owned())
            };

            session.disambiguate(&model.seen_tmux_names);
            model.sessions.push(session.into());
        }

        // Attach information about workspaces seen, to help disambiguate future new workspaces
        // created by the tool.
        for (root, workspace) in &model.workspaces {
            let Some(workspace) = workspace else {
                continue;
            };

            let root = workspace.default.as_ref().unwrap_or(root);
            let name = workspace.name.as_deref().unwrap_or(jj::DEFAULT_WORKSPACE);

            model
                .seen_workspaces
                .entry(root.to_owned())
                .or_default()
                .insert(name.to_owned());
        }

        Ok(model)
    }

    /// Construct the dynamic "new session" candidate for the current query and repo context.
    pub(super) fn new_session(&self, query: &str, repo: Option<&Path>) -> Option<Session> {
        if query.is_empty() {
            return None;
        }

        let base = match repo {
            None => Base::Cwd(None),
            Some(repo) => match self.workspaces.get(repo) {
                None => Base::Cwd(Some(repo.to_owned())),
                Some(workspace) => Base::Repo(
                    workspace
                        .as_ref()
                        .and_then(|w| w.default.clone())
                        .unwrap_or_else(|| repo.to_owned()),
                ),
            },
        };

        let empty = BTreeSet::new();
        let siblings = match &base {
            Base::Repo(default) => self.seen_workspaces.get(default).unwrap_or(&empty),
            Base::Cwd(_) => &empty,
        };

        let mut session = NewKind::new(query, base);
        session.disambiguate(&self.seen_tmux_names, siblings);
        Some(session.into())
    }

    /// Return the discovered sessions.
    pub(super) fn sessions(&self) -> &[Session] {
        &self.sessions
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
