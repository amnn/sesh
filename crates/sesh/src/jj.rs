// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Helpers for interacting with `jj` repositories.

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Context as _;
use anyhow::ensure;
use which::which;

/// Validate that `jj` is available on `$PATH`.
pub fn ensure() -> anyhow::Result<()> {
    ensure!(which("jj").is_ok(), "'jj' not found in PATH");
    Ok(())
}

/// Fetch `jj log` output from the repository at `repo`.
pub fn log(repo: &Path) -> anyhow::Result<String> {
    let output = Command::new("jj")
        .arg("log")
        .arg("-R")
        .arg(repo)
        .arg("--color")
        .arg("always")
        .output()
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
            {
                repos.insert(path);
            }
        }
    }

    Ok(repos)
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
        let nested = repo.join("src/nested");

        fs::create_dir_all(repo.join(".jj")).unwrap();
        fs::create_dir_all(&nested).unwrap();

        assert_eq!(repo_root(&nested), Some(repo));
    }

    #[test]
    fn returns_none_when_path_is_not_in_repo() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("plain");
        fs::create_dir_all(&path).unwrap();

        assert_eq!(repo_root(&path), None);
    }
}
