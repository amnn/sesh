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

/// The conventional workspace name created by `jj git init`.
pub const DEFAULT_WORKSPACE: &str = "default";

/// Create a new workspace in `destination`, named `name`, with working copy based on `trunk()`.
pub async fn add_workspace(repo: &Path, destination: &Path, name: &str) -> anyhow::Result<()> {
    let output = Command::new("jj")
        .args(["workspace", "add"])
        .arg("-R")
        .arg(repo)
        .arg("--name")
        .arg(name)
        .arg("--revision")
        .arg("trunk()")
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

/// Fetch `jj log` output from the repository at `repo`.
pub async fn log(repo: &Path) -> anyhow::Result<String> {
    let output = Command::new("jj")
        .arg("log")
        .arg("-R")
        .arg(repo)
        .arg("--color")
        .arg("always")
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
pub fn repos(globs: &[String]) -> anyhow::Result<BTreeSet<PathBuf>> {
    let mut repos = BTreeSet::new();
    for pattern in globs {
        for path in glob::glob(pattern).with_context(|| format!("invalid glob: '{pattern}'"))? {
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

/// List all workspaces registered for the repository containing `repo`.
///
/// The default workspace name is normalized to `None`; named workspaces are `Some(name)`.
pub async fn workspaces(repo: &Path) -> anyhow::Result<BTreeMap<Option<String>, Option<PathBuf>>> {
    let output = Command::new("jj")
        .args(["workspace", "list"])
        .arg("-R")
        .arg(repo)
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

    #[test]
    fn finds_repo_root_from_nested_directory() {
        let temp = tempdir().unwrap();
        let repo = temp.path().join("repo");
        let nested = repo.join("src").join("nested");

        fs::create_dir_all(repo.join(".jj")).unwrap();
        fs::create_dir_all(&nested).unwrap();

        assert_eq!(repo_root(&nested), Some(repo));
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
}
