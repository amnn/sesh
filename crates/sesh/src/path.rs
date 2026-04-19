// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Path helpers for UI-oriented path formatting.

use std::env;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use once_cell::sync::Lazy;

static HOME_DIR: Lazy<Option<PathBuf>> =
    Lazy::new(|| env::home_dir().map(|home| home.canonicalize().unwrap_or(home)));

/// Path display helpers that apply the project's UI-specific shortening rules.
pub(crate) trait TruncatedExt {
    /// Return a path with compacted parent components.
    fn compact(&self) -> PathBuf;

    /// Return a path with the home directory shortened to `~`.
    fn truncated(&self) -> PathBuf;
}

impl TruncatedExt for Path {
    fn compact(&self) -> PathBuf {
        let mut path = PathBuf::new();
        let mut parts = self.components().peekable();

        while let Some(part) = parts.next() {
            if parts.peek().is_some()
                && let Component::Normal(name) = part
                && let Some(init) = name.to_string_lossy().chars().next()
            {
                path.push(init.to_string());
            } else {
                path.push(part.as_os_str())
            }
        }

        path
    }

    fn truncated(&self) -> PathBuf {
        let Some(home) = HOME_DIR.as_deref() else {
            return self.to_path_buf();
        };

        if let Ok(path) = self.strip_prefix(home) {
            PathBuf::from("~").join(path)
        } else {
            self.to_path_buf()
        }
    }
}

impl TruncatedExt for PathBuf {
    fn compact(&self) -> PathBuf {
        self.as_path().compact()
    }

    fn truncated(&self) -> PathBuf {
        self.as_path().truncated()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::TruncatedExt as _;

    #[test]
    fn compact_contracts_absolute_intermediate_components() {
        let path = PathBuf::from("/tmp/foo/bar");

        assert_eq!(path.compact(), PathBuf::from("/t/f/bar"));
    }

    #[test]
    fn compacts_truncated_home_relative_paths() {
        let Some(home) = super::HOME_DIR.as_ref() else {
            return;
        };

        let path = home.join("Code/foo/bar");
        assert_eq!(path.truncated().compact(), PathBuf::from("~/C/f/bar"));
    }

    #[test]
    fn leaves_non_home_paths_unchanged() {
        let path = PathBuf::from("/tmp/repo");

        assert_eq!(path.truncated(), PathBuf::from("/tmp/repo"));
    }

    #[test]
    fn truncates_paths_under_cached_home() {
        let Some(home) = super::HOME_DIR.as_ref() else {
            return;
        };

        let path = home.join("repo");
        assert_eq!(path.truncated(), PathBuf::from("~/repo"));
    }
}
