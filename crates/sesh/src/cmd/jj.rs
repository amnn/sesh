// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Helpers for interacting with `jj` repositories.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::ensure;
use tokio::process::Command;
use which::which;

use crate::path::expand_home;

/// The default revision used as the parent for newly-created workspaces.
pub const DEFAULT_BASE_REVSET: &str = "trunk()";

/// The conventional workspace name created by `jj git init`.
pub const DEFAULT_WORKSPACE: &str = "default";

/// Create a new workspace in `destination`, named `name`, with working copy based on `revision`.
pub async fn add_workspace(
    repo: &Path,
    destination: &Path,
    name: &str,
    revision: &str,
) -> anyhow::Result<()> {
    let output = Command::new("jj")
        .args(["workspace", "add"])
        .arg("-R")
        .arg(repo)
        .arg("--name")
        .arg(name)
        .arg("--revision")
        .arg(revision)
        .arg(destination)
        .output()
        .await
        .with_context(|| {
            format!(
                "failed to run 'jj workspace add' for repo '{}'",
                repo.display()
            )
        })?;

    ensure!(
        output.status.success(),
        "error running 'jj workspace add': {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );

    Ok(())
}

/// Validate that `jj` is available on `$PATH`.
pub fn ensure() -> anyhow::Result<()> {
    ensure!(which("jj").is_ok(), "'jj' not found in PATH");
    Ok(())
}

/// Forget the workspace named `name` in the repository containing `repo`.
pub async fn forget_workspace(repo: &Path, name: &str) -> anyhow::Result<()> {
    let output = Command::new("jj")
        .args(["workspace", "forget"])
        .arg("-R")
        .arg(repo)
        .arg("--ignore-working-copy")
        .arg("--")
        .arg(name)
        .output()
        .await
        .with_context(|| {
            format!(
                "failed to run 'jj workspace forget' for repo '{}'",
                repo.display()
            )
        })?;

    ensure!(
        output.status.success(),
        "error running 'jj workspace forget': {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );

    Ok(())
}

/// Fetch `jj log` output from the repository at `repo`.
pub async fn log(repo: &Path) -> anyhow::Result<String> {
    let output = Command::new("jj")
        .arg("log")
        .arg("-R")
        .arg(repo)
        .arg("--ignore-working-copy")
        .arg("--no-pager")
        .args(["--config", "ui.graph.style=curved"])
        .args(["--color", "always"])
        .args(["--template", "builtin_log_compact"])
        .output()
        .await
        .with_context(|| format!("failed to run 'jj log' for repo '{}'", repo.display()))?;

    Ok(if output.status.success() {
        String::from_utf8_lossy(&output.stdout).into_owned()
    } else {
        String::from_utf8_lossy(&output.stderr).into_owned()
    })
}

/// Discover the nearest enclosing jj repository for `path`.
pub fn repo_root(path: &Path) -> Option<PathBuf> {
    for ancestor in path.ancestors() {
        if ancestor.join(".jj").is_dir() {
            return Some(ancestor.to_path_buf());
        }
    }

    None
}

/// Discover valid jj repositories from directories matching `globs`.
///
/// A leading `~` path component in a glob is expanded to the user's home directory. Returns an
/// error if a glob is invalid or a leading `~` cannot be expanded into a UTF-8 path.
pub fn repos(globs: &[String]) -> anyhow::Result<BTreeSet<PathBuf>> {
    let mut repos = BTreeSet::new();
    for pattern in globs {
        let expanded = expand_home(pattern);
        let expanded = expanded.to_str().context("invalid glob")?;
        for path in glob::glob(expanded).with_context(|| format!("invalid glob: '{pattern}'"))? {
            if let Ok(path) = path
                && path.join(".jj").is_dir()
                && let Ok(path) = path.canonicalize()
            {
                repos.insert(path);
            }
        }
    }

    Ok(repos)
}

/// Fetch template-only `jj show` output for `rev` in the repository at `repo`.
pub async fn show(repo: &Path, rev: &str, template: &str) -> anyhow::Result<String> {
    let output = Command::new("jj")
        .arg("show")
        .arg("-R")
        .arg(repo)
        .args(["-r", rev])
        .arg("--ignore-working-copy")
        .arg("--no-pager")
        .arg("--no-patch")
        .args(["--color", "never"])
        .args(["--template", template])
        .output()
        .await
        .with_context(|| {
            format!(
                "failed to run 'jj show' for revision '{rev}' in repo '{}'",
                repo.display()
            )
        })?;

    ensure!(
        output.status.success(),
        "error running 'jj show': {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// List all workspaces registered for the repository containing `repo`.
///
/// The default workspace name is normalized to `None`; named workspaces are `Some(name)`.
pub async fn workspaces(repo: &Path) -> anyhow::Result<BTreeMap<Option<String>, Option<PathBuf>>> {
    let output = Command::new("jj")
        .args(["workspace", "list"])
        .arg("-R")
        .arg(repo)
        .arg("--ignore-working-copy")
        .arg("--no-pager")
        .args(["--color", "never"])
        .args(["--template", "name ++ '\t' ++ root ++ '\n'"])
        .output()
        .await
        .with_context(|| {
            format!(
                "failed to run 'jj workspace list' for repo '{}'",
                repo.display()
            )
        })?;

    ensure!(
        output.status.success(),
        "error running 'jj workspace list': {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );

    let mut workspaces = BTreeMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some((name, root)) = line.split_once('\t') else {
            continue;
        };

        let root = if root.starts_with("<Error:") {
            None
        } else {
            Some(PathBuf::from(root))
        };

        let normalized = (name != DEFAULT_WORKSPACE).then(|| name.to_owned());
        workspaces.insert(normalized, root);
    }

    Ok(workspaces)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn creates_workspace_from_explicit_revision() {
        let temp = tempdir().unwrap();
        let default = temp.path().join("repo");
        let workspace = temp.path().join("repo.feature");

        let output = Command::new("jj")
            .args(["git", "init"])
            .arg(&default)
            .output()
            .await
            .unwrap();
        assert!(output.status.success());

        let output = Command::new("jj")
            .arg("describe")
            .arg("-R")
            .arg(&default)
            .args(["-m", "base"])
            .output()
            .await
            .unwrap();
        assert!(output.status.success());

        add_workspace(&default, &workspace, "feature", "@")
            .await
            .unwrap();

        let output = Command::new("jj")
            .arg("log")
            .arg("-R")
            .arg(&workspace)
            .arg("--ignore-working-copy")
            .args(["--no-graph", "--color", "never"])
            .args(["-r", "@-"])
            .args(["--template", "description"])
            .output()
            .await
            .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "base");
    }

    #[test]
    fn finds_repo_root_from_nested_directory() {
        let temp = tempdir().unwrap();
        let repo = temp.path().join("repo");
        let nested = repo.join("src").join("nested");

        fs::create_dir_all(repo.join(".jj")).unwrap();
        fs::create_dir_all(&nested).unwrap();

        assert_eq!(repo_root(&nested), Some(repo));
    }

    #[tokio::test]
    async fn forgets_named_workspace() {
        let temp = tempdir().unwrap();
        let default = temp.path().join("repo");
        let workspace = temp.path().join("repo.feature");

        let output = Command::new("jj")
            .args(["git", "init"])
            .arg(&default)
            .output()
            .await
            .unwrap();
        assert!(output.status.success());

        let output = Command::new("jj")
            .args(["workspace", "add"])
            .arg("-R")
            .arg(&default)
            .arg("--name")
            .arg("feature")
            .arg(&workspace)
            .output()
            .await
            .unwrap();
        assert!(output.status.success());

        forget_workspace(&workspace, "feature").await.unwrap();

        assert!(workspace.exists());
        assert!(
            !workspaces(&default)
                .await
                .unwrap()
                .contains_key(&Some("feature".to_owned()))
        );
    }

    #[tokio::test]
    async fn logs_with_curved_graph_style_despite_repo_config() {
        let temp = tempdir().unwrap();
        let repo = temp.path().join("repo");

        let output = Command::new("jj")
            .args(["git", "init"])
            .arg(&repo)
            .output()
            .await
            .unwrap();
        assert!(output.status.success());

        let output = Command::new("jj")
            .args(["config", "set"])
            .arg("-R")
            .arg(&repo)
            .arg("--repo")
            .args(["ui.graph.style", "ascii"])
            .output()
            .await
            .unwrap();
        assert!(output.status.success());

        let output = log(&repo).await.unwrap();

        assert!(output.contains('◆'), "log output: {output:?}");
        assert!(!output.contains("+  zzzzzzzz"), "log output: {output:?}");
    }

    #[test]
    fn returns_canonical_repo_paths_from_globs() {
        let temp = tempdir().unwrap();
        let repo = temp.path().join("repo");

        fs::create_dir_all(repo.join(".jj")).unwrap();

        let pattern = repo.display().to_string();
        assert_eq!(
            repos(&[pattern]).unwrap(),
            BTreeSet::from([repo.canonicalize().unwrap()])
        );
    }

    #[test]
    fn returns_none_when_path_is_not_in_repo() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("plain");
        fs::create_dir_all(&path).unwrap();

        assert_eq!(repo_root(&path), None);
    }

    #[tokio::test]
    async fn shows_non_empty_revision_with_template_without_patch() {
        let temp = tempdir().unwrap();
        let repo = temp.path().join("repo");

        let output = Command::new("jj")
            .args(["git", "init"])
            .arg(&repo)
            .output()
            .await
            .unwrap();
        assert!(output.status.success());

        fs::write(repo.join("file"), "contents").unwrap();
        let output = Command::new("jj")
            .arg("describe")
            .arg("-R")
            .arg(&repo)
            .args(["-m", "non-empty"])
            .output()
            .await
            .unwrap();
        assert!(output.status.success());

        let template = concat!(
            r#"change_id.short() ++ "\t" ++ self.contained_in("trunk()") ++ "\t" ++ "#,
            r#"local_bookmarks ++ "\t" ++ remote_bookmarks ++ "\n""#,
        );

        let output = show(&repo, "@", template).await.unwrap();
        let lines: Vec<_> = output.lines().collect();

        assert_eq!(lines.len(), 1, "output: {output:?}");
        let fields: Vec<_> = lines[0].split('\t').collect();
        assert_eq!(fields.len(), 4);
        assert_eq!(fields[0].len(), 12);
        assert!(fields[0].chars().all(|c| c.is_ascii_lowercase()));
        assert!(matches!(fields[1], "true" | "false"));
        assert_eq!(fields[2], "");
        assert_eq!(fields[3], "");
    }
}
