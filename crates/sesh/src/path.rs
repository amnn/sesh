// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Path helpers for UI-oriented path formatting.

use std::env;
use std::ffi::OsStr;
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

    /// Return the parent and final path component of this path.
    fn split_last(&self) -> (&Path, &OsStr);

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

    fn split_last(&self) -> (&Path, &OsStr) {
        let parent = self.parent().unwrap_or_else(|| Path::new(""));
        let base = self
            .components()
            .next_back()
            .map_or_else(|| OsStr::new(""), |component| component.as_os_str());

        (parent, base)
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

    fn split_last(&self) -> (&Path, &OsStr) {
        self.as_path().split_last()
    }

    fn truncated(&self) -> PathBuf {
        self.as_path().truncated()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn compact_contracts_absolute_intermediate_components() {
        let path = PathBuf::from("/tmp/foo/bar");

        assert_eq!(path.compact(), PathBuf::from("/t/f/bar"));
    }

    #[test]
    fn compacts_truncated_home_relative_paths() {
        let Some(home) = HOME_DIR.as_ref() else {
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
    fn splits_compact_display_path_into_prefix_and_basename() {
        let Some(home) = HOME_DIR.as_ref() else {
            return;
        };

        let path = home.join("Code/foo/bar").truncated().compact();
        assert_eq!(
            path.split_last(),
            (PathBuf::from("~/C/f").as_path(), OsStr::new("bar"))
        );
    }

    #[test]
    fn splits_empty_path_into_empty_parent_and_basename() {
        let path = PathBuf::from("");

        assert_eq!(path.split_last(), (Path::new(""), OsStr::new("")));
    }

    #[test]
    fn splits_parent_dir_as_basename() {
        let path = PathBuf::from("foo/..");

        assert_eq!(
            path.split_last(),
            (PathBuf::from("foo").as_path(), OsStr::new(".."))
        );
    }

    #[test]
    fn splits_root_path_into_empty_parent_and_root_basename() {
        let path = PathBuf::from("/");

        assert_eq!(path.split_last(), (Path::new(""), OsStr::new("/")));
    }

    #[test]
    fn truncates_paths_under_cached_home() {
        let Some(home) = HOME_DIR.as_ref() else {
            return;
        };

        let path = home.join("repo");
        assert_eq!(path.truncated(), PathBuf::from("~/repo"));
    }
}
