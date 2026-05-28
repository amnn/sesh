// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Session domain model.

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;

use crate::cmd::jj;
use crate::cmd::tmux;
use crate::model::picker::Pickable;
use crate::path::TruncatedExt as _;

pub(crate) const DELIM_SUFFIX: &str = "~";
pub(crate) const NAME_WIDTH: usize = 40;

const DELIM_WORKSPACE: &str = "/";
const TMUX_REPO_OPTION: &str = "@sesh.repo";

/// A tmux session or potential session.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Session(Kind);

/// The base used when creating a new session.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Base {
    /// Create a jj workspace from this repository information.
    Repo(Repo),
    /// Create a tmux session at this working directory, or the process cwd if absent.
    Cwd(Option<PathBuf>),
}

/// A live tmux session; repo metadata is display-only.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct LiveKind {
    name: String,
    repo: Option<PathBuf>,
    alerts: Vec<String>,
    flagged: bool,
    can_delete: bool,
}

/// A new session, optionally backed by a jj workspace to create from a repository base.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct NewKind {
    name: String,
    base: Base,
    suffix: Option<String>,
}

/// Repository information used while constructing workspace-backed sessions.
///
/// `source` is the selected checkout whose workspace metadata is inspected. `default` is the
/// resolved default checkout used for workspace naming and placement. `revision` is the jj revset
/// used as the new workspace base.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Repo {
    /// Selected repository or workspace checkout used to look up workspace metadata.
    source: PathBuf,
    /// Default checkout used to derive sibling workspace names and paths.
    default: PathBuf,
    /// jj revset used as the base revision for new workspaces.
    revision: String,
}

/// Session for a repository or workspace checkout that already exists.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct RepoKind {
    workspace: Option<String>,
    default: PathBuf,
    path: PathBuf,
    suffix: Option<String>,
    can_delete: bool,
}

/// Backing kind for a picker session.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum Kind {
    Live(LiveKind),
    New(NewKind),
    Repo(RepoKind),
}

impl Session {
    /// Return whether this entry represents a currently live tmux session that can be closed.
    pub fn can_close(&self) -> bool {
        matches!(&self.0, Kind::Live(_))
    }

    /// Return whether this entry can be deleted.
    pub fn can_delete(&self) -> bool {
        match &self.0 {
            Kind::Live(kind) => kind.can_delete,
            Kind::New(_) => false,
            Kind::Repo(kind) => kind.can_delete,
        }
    }

    /// Close this session without deleting any attached workspace.
    pub async fn close(&self) -> anyhow::Result<()> {
        match &self.0 {
            Kind::Live(kind) => kind.close().await,
            Kind::New(_) | Kind::Repo(_) => Ok(()),
        }
    }

    /// Create this session if needed without switching the current tmux client.
    pub async fn create(&self, cwd: &Path, setup: &str) -> anyhow::Result<()> {
        self.ensure_tmux(cwd, setup).await
    }

    /// Return this session's manual flag state, if this entry can be flagged.
    pub fn flag(&self) -> Option<bool> {
        match &self.0 {
            Kind::Live(kind) => Some(kind.flagged),
            Kind::New(_) | Kind::Repo(_) => None,
        }
    }

    /// Return the session name.
    pub fn name(&self) -> String {
        match &self.0 {
            Kind::Live(kind) => kind.name(),
            Kind::New(kind) => kind.name(),
            Kind::Repo(kind) => kind.name(),
        }
    }

    /// Return the repository attached to this session, if any.
    pub fn repo(&self) -> Option<PathBuf> {
        match &self.0 {
            Kind::Live(kind) => kind.repo(),
            Kind::New(kind) => kind.repo(),
            Kind::Repo(kind) => kind.repo(),
        }
    }

    /// Switch the current tmux client to this session, creating the session first if needed.
    pub async fn switch(&self, cwd: &Path, setup: &str) -> anyhow::Result<()> {
        self.create(cwd, setup).await?;
        tmux::switch_client(&self.switch_target()).await
    }

    /// Toggle this session's persisted manual flag.
    pub async fn toggle_flag(&self) -> anyhow::Result<()> {
        match &self.0 {
            Kind::Live(kind) => kind.toggle_flag().await,
            Kind::New(_) | Kind::Repo(_) => Ok(()),
        }
    }

    /// Return the live tmux alert windows for this session, if any.
    pub(crate) fn alerts(&self) -> &[String] {
        match &self.0 {
            Kind::Live(kind) => &kind.alerts,
            Kind::New(_) | Kind::Repo(_) => &[],
        }
    }

    /// Return the repository whose log should be shown in the preview pane.
    pub(crate) fn preview_repo(&self) -> Option<PathBuf> {
        match &self.0 {
            Kind::Live(kind) => kind.repo(),
            Kind::New(kind) => kind.preview_repo(),
            Kind::Repo(kind) => kind.repo(),
        }
    }

    /// Ensure the tmux session we are switching to is ready.
    async fn ensure_tmux(&self, cwd: &Path, setup: &str) -> anyhow::Result<()> {
        match &self.0 {
            Kind::Live(_) => Ok(()),
            Kind::New(kind) => kind.ensure_tmux(cwd, setup).await,
            Kind::Repo(kind) => kind.ensure_tmux(setup).await,
        }
    }

    /// Return the tmux target for switching to this session.
    fn switch_target(&self) -> String {
        let session = self.name();
        let Kind::Live(LiveKind { alerts, .. }) = &self.0 else {
            return session;
        };

        if let Some(window) = alerts.first() {
            format!("{session}:{window}")
        } else {
            session
        }
    }
}

impl LiveKind {
    /// Construct a potential session from information extracted from `tmux`.
    ///
    /// `name` is a tmux session name, `repo` is an optional path to a jj repository attached as a
    /// user-option on the tmux session, `alerts` is a list of windows in the session that have an
    /// active bell alert, `flagged` indicates whether the user has manually flagged the session,
    /// and `can_delete` indicates whether deletion can remove a named jj workspace.
    pub(crate) fn new(
        name: String,
        repo: Option<PathBuf>,
        alerts: Vec<String>,
        flagged: bool,
        can_delete: bool,
    ) -> Self {
        Self {
            name,
            repo,
            alerts,
            flagged,
            can_delete,
        }
    }

    /// Close the live tmux session without deleting any attached workspace.
    async fn close(&self) -> anyhow::Result<()> {
        tmux::kill_session(&self.name).await
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn repo(&self) -> Option<PathBuf> {
        self.repo.clone()
    }

    /// Toggle the persistent tmux user option that stores this session's manual flag.
    async fn toggle_flag(&self) -> anyhow::Result<()> {
        tmux::set_flag(&self.name, !self.flagged).await
    }
}

impl NewKind {
    /// Construct a new potential session from a query and session base.
    pub(crate) fn new(name: &str, base: Base) -> Self {
        Self {
            name: sanitize(name),
            base,
            suffix: None,
        }
    }

    /// Tweak session's suffix until its tmux session name, workspace name, and repo path are all
    /// unique.
    ///
    /// `sessions` is the list of all tmux sessions found on startup, and `siblings` is the set of
    /// other workspaces associated with the same default repo as this session.
    pub(crate) fn disambiguate(
        &mut self,
        sessions: &BTreeSet<String>,
        workspaces: &BTreeSet<String>,
    ) {
        let mut i = 1;
        while sessions.contains(&self.name())
            || self.workspace().is_some_and(|w| workspaces.contains(&w.1))
            || self.repo().is_some_and(|r| r.exists())
        {
            self.suffix = Some(i.to_string());
            i += 1;
        }
    }

    /// Ensure the tmux session for this new session exists.
    async fn ensure_tmux(&self, cwd: &Path, setup: &str) -> anyhow::Result<()> {
        let target = self.name();
        let repo = self.repo();
        let cwd = match &self.base {
            Base::Repo(_) => repo.clone().context("missing repo")?,
            Base::Cwd(Some(cwd)) => cwd.clone(),
            Base::Cwd(None) => cwd.to_owned(),
        };

        self.ensure_workspace().await?;
        tmux::new_session(&target, &cwd).await?;

        if let Some(repo) = self.repo() {
            tmux::set_option(&target, TMUX_REPO_OPTION, &repo).await?;
        }

        tmux::run_shell(&format!("{target}:0"), &cwd, setup).await
    }

    /// Ensure the jj workspace for this new session exists, when it is repo-backed.
    async fn ensure_workspace(&self) -> anyhow::Result<()> {
        let Some((default, workspace, revision)) = self.workspace() else {
            return Ok(());
        };

        let destination = self
            .repo()
            .context("workspace-backed session is missing a destination")?;

        jj::add_workspace(default, &destination, &workspace, revision).await
    }

    /// The tmux session name for the new session.
    fn name(&self) -> String {
        let base = match &self.base {
            Base::Repo(base) => Some(base.default()),
            Base::Cwd(_) => None,
        };

        workspace_session_name(base, Some(&self.name), self.suffix.as_deref())
    }

    /// The repository whose log should be shown before this session's workspace exists.
    fn preview_repo(&self) -> Option<PathBuf> {
        match &self.base {
            Base::Repo(base) => Some(base.default().to_owned()),
            Base::Cwd(_) => None,
        }
    }

    /// The repository associated with this session. Disambiguation ensures this path does not
    /// collide with an existing repo.
    fn repo(&self) -> Option<PathBuf> {
        let (default, workspace, _) = self.workspace()?;
        Some(default.with_added_extension(&workspace))
    }

    /// This session's workspace name. Disambiguation ensures this name does not collide with an
    /// existing workspace name.
    fn workspace(&self) -> Option<(&Path, String, &str)> {
        let Base::Repo(base) = &self.base else {
            return None;
        };

        let mut workspace = self.name.clone();
        if let Some(suffix) = &self.suffix {
            workspace.push_str(DELIM_SUFFIX);
            workspace.push_str(suffix);
        }

        Some((base.default(), workspace, base.revision()))
    }
}

impl Repo {
    /// Package repository information with the default base revision.
    pub(crate) fn new(source: PathBuf) -> Self {
        Self {
            source: source.clone(),
            default: source,
            revision: jj::DEFAULT_BASE_REVSET.to_owned(),
        }
    }

    /// Return the default workspace checkout that names new workspaces.
    pub(crate) fn default(&self) -> &Path {
        &self.default
    }

    /// Return the selected base revision expression.
    pub(crate) fn revision(&self) -> &str {
        &self.revision
    }

    /// Return the repository or workspace path this information applies to.
    pub(crate) fn source(&self) -> &Path {
        &self.source
    }

    /// Return a copy of this repo with the default workspace checkout overridden.
    pub(crate) fn with_default(&self, default: PathBuf) -> Self {
        Self {
            source: self.source.clone(),
            default,
            revision: self.revision.clone(),
        }
    }
}

impl RepoKind {
    /// Construct a potential session from an existing repository or workspace checkout.
    pub(crate) fn new(
        workspace: Option<&str>,
        default: PathBuf,
        path: PathBuf,
        can_delete: bool,
    ) -> Self {
        Self {
            workspace: workspace.map(sanitize),
            default,
            path,
            suffix: None,
            can_delete,
        }
    }

    /// Tweak the session's `suffix` until `session.name()` does not collide with any live tmux
    /// session names already seen.
    pub(crate) fn disambiguate(&mut self, sessions: &BTreeSet<String>) {
        let mut i = 1;
        while sessions.contains(&self.name()) {
            self.suffix = Some(i.to_string());
            i += 1;
        }
    }

    /// Ensure the tmux session for this repository checkout exists.
    async fn ensure_tmux(&self, setup: &str) -> anyhow::Result<()> {
        let target = self.name();
        tmux::new_session(&target, &self.path).await?;
        tmux::set_option(&target, TMUX_REPO_OPTION, &self.path).await?;
        tmux::run_shell(&format!("{target}:0"), &self.path, setup).await
    }

    /// The tmux session name for a session attached to this existing repo/workspace.
    fn name(&self) -> String {
        workspace_session_name(
            Some(&self.default),
            self.workspace.as_deref(),
            self.suffix.as_deref(),
        )
    }

    /// The repository associated with this session.
    fn repo(&self) -> Option<PathBuf> {
        Some(self.path.clone())
    }
}

impl From<LiveKind> for Session {
    fn from(kind: LiveKind) -> Self {
        Self(Kind::Live(kind))
    }
}

impl From<NewKind> for Session {
    fn from(kind: NewKind) -> Self {
        Self(Kind::New(kind))
    }
}

impl From<RepoKind> for Session {
    fn from(kind: RepoKind) -> Self {
        Self(Kind::Repo(kind))
    }
}

impl Pickable for Session {
    fn text(&self) -> String {
        let Some(repo) = self.repo() else {
            return self.name();
        };

        format!(
            "{:<NAME_WIDTH$} {}",
            self.name(),
            repo.truncated().display()
        )
    }
}

/// Make the name safe for use as a tmux session name and a workspace name.
pub(crate) fn sanitize(name: &str) -> String {
    let strip = |c: char| c.is_control() || [' ', ':', '.', '/', '\\', '-'].contains(&c);
    let mut cs = name.trim_matches(strip).chars().peekable();

    let mut sanitized = String::new();
    while let Some(c) = cs.peek() {
        if !strip(*c) {
            sanitized.push(cs.next().unwrap());
            continue;
        }

        while cs.peek().is_some_and(|c| strip(*c)) {
            cs.next();
        }

        sanitized.push('-')
    }

    sanitized
}

/// Derive a workspace-aware tmux session name.
///
/// Each component is optional, but one of `base` or `workspace` is expected to be present. The
/// resulting name takes the form `{base}/{workspace}~{suffix}`. Each part's prefix is omitted if
/// the part itself is omitted or it is the first part.
fn workspace_session_name(
    base: Option<&Path>,
    workspace: Option<&str>,
    suffix: Option<&str>,
) -> String {
    let mut name = String::new();
    if let Some(base) = base {
        let base = base.file_name().expect("non-canonical");
        let base = sanitize(&base.to_string_lossy());
        name.push_str(&base);
    }

    if let Some(workspace) = workspace {
        if !name.is_empty() {
            name.push_str(DELIM_WORKSPACE);
        }

        name.push_str(workspace);
    }

    if let Some(suffix) = suffix {
        if !name.is_empty() {
            name.push_str(DELIM_SUFFIX);
        }

        name.push_str(suffix);
    }

    name
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn new_workspace_sessions_derive_names_and_paths() {
        let temp = tempdir().unwrap();
        let default = temp.path().join("repo");
        let session = NewKind::new("feature", Base::Repo(Repo::new(default)));

        assert_eq!(session.name(), "repo/feature");
        assert_eq!(session.repo(), Some(temp.path().join("repo.feature")));
    }

    #[test]
    fn workspace_session_names_are_sanitized() {
        let session = NewKind::new(
            "feature: one.two/path\\name\n",
            Base::Repo(Repo::new(PathBuf::from("repo.default"))),
        );

        assert_eq!(session.name(), "repo-default/feature-one-two-path-name");
    }
}
