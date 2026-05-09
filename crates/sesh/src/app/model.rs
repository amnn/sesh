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
    workspaces: BTreeMap<PathBuf, Workspace>,
}

/// Workspace metadata for a discovered repository.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Workspace {
    /// Workspace name reported by `jj workspace list` for this repository root.
    name: String,
    /// Root path for the default workspace in the same jj repository, when available.
    default: Option<PathBuf>,
}

impl Model {
    /// Discover live tmux sessions and repository-backed candidate sessions.
    pub async fn discover(repos: &[String]) -> anyhow::Result<Self> {
        let mut model = Self::default();
        let mut tmux_repos = BTreeSet::new();

        // Add all the live sessions from tmux.
        for (name, info) in tmux::sessions().await? {
            model.seen_names.insert(name.clone());
            tmux_repos.extend(info.repo.clone());

            let session = Session::from_tmux(name, info.repo, info.alerts);
            model.sessions.push(session);
        }

        let repos = jj::repos(repos)?;
        model.workspaces = workspaces(repos.iter().chain(&tmux_repos).map(PathBuf::as_path)).await;

        // Add an entry for every repo found, as long as it's not already associated with a
        // live tmux session.
        for repo in repos {
            if tmux_repos.contains(&repo) {
                continue;
            }

            let Some(name) = model.repo_session_name(&repo) else {
                continue;
            };

            let mut session = Session::from_repo(name, repo);
            model.disambiguate(&mut session);
            model.sessions.push(session);
        }

        Ok(model)
    }

    /// Construct the dynamic "new session" candidate for the current query and repo context.
    pub(super) fn new_session(&self, query: &str, repo: Option<&Path>) -> Option<Session> {
        if query.is_empty() {
            return None;
        }

        let name = self.new_session_name(query, repo)?;
        let mut session = Session::new(name, repo.map(Path::to_owned));
        self.disambiguate(&mut session);

        Some(session)
    }

    /// Return the discovered sessions.
    pub(super) fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    /// Tweak `session`'s `suffix` until `session.name()` does not collide with any live tmux
    /// session name already seen.
    fn disambiguate(&self, session: &mut Session) {
        let mut i = 1;
        while self.seen_names.contains(&session.name()) {
            session.set_suffix(i.to_string());
            i += 1;
        }
    }

    /// Build the dynamic "new session" name for a query and optional repository context.
    fn new_session_name(&self, query: &str, repo: Option<&Path>) -> Option<String> {
        let default = repo
            .and_then(|repo| self.workspaces.get(repo))
            .and_then(|workspace| workspace.default.as_deref());

        workspace_session_name(query, default)
    }

    /// Build the session name for a repository-backed candidate.
    fn repo_session_name(&self, repo: &Path) -> Option<String> {
        if let Some(workspace) = self.workspaces.get(repo) {
            workspace_session_name(&workspace.name, workspace.default.as_deref())
        } else {
            workspace_session_name("default", Some(repo))
        }
    }
}

/// Return a tmux-safe session name component.
fn sanitize(name: &str) -> String {
    name.replace([' ', ':', '.'], "-")
}

/// Build the unsuffixed workspace-aware session name.
fn workspace_session_name(workspace: &str, default: Option<&Path>) -> Option<String> {
    let prefix = default
        .and_then(Path::file_name)
        .map(|n| sanitize(&n.to_string_lossy()));

    match (prefix, workspace) {
        (None, "default") => None,
        (None, workspace) => Some(sanitize(workspace)),
        (Some(prefix), "default") => Some(prefix),
        (Some(prefix), workspace) => Some(format!("{prefix}/{}", sanitize(workspace))),
    }
}

/// Discover workspace metadata for every workspace associated with each repository.
async fn workspaces<'a>(repos: impl IntoIterator<Item = &'a Path>) -> BTreeMap<PathBuf, Workspace> {
    let mut tasks: FuturesUnordered<_> = repos.into_iter().map(jj::workspaces).collect();

    let mut info = BTreeMap::new();
    while let Some(Ok(ws)) = tasks.next().await {
        let default = ws.get("default").cloned().flatten();
        for (name, root) in ws {
            let Some(root) = root else {
                continue;
            };

            info.insert(
                root,
                Workspace {
                    name,
                    default: default.clone(),
                },
            );
        }
    }

    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_session_names_are_sanitized() {
        let name = workspace_session_name("feature: one.two", Some(Path::new("/tmp/repo.default")));

        assert_eq!(name.as_deref(), Some("repo-default/feature--one-two"));
    }

    #[test]
    fn workspace_session_names_omit_default_workspace_name() {
        let name = workspace_session_name("default", Some(Path::new("/tmp/repo.default")));

        assert_eq!(name.as_deref(), Some("repo-default"));
    }

    #[test]
    fn workspace_session_names_omit_missing_default_prefix() {
        let name = workspace_session_name("feature: one.two", None);

        assert_eq!(name.as_deref(), Some("feature--one-two"));
    }
}
