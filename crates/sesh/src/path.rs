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
                && let Some(init) = compact_component(name)
            {
                path.push(init);
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

/// Return the compact display form of a single path component.
fn compact_component(name: &OsStr) -> Option<String> {
    let name = name.to_string_lossy();
    let mut chars = name.chars();

    match chars.next()? {
        '.' => chars.next().map(|next| format!(".{next}")),
        first => Some(first.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::MAIN_SEPARATOR_STR;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn compact_contracts_absolute_intermediate_components() {
        let path = PathBuf::from(MAIN_SEPARATOR_STR)
            .join("tmp")
            .join("foo")
            .join("bar");
        let expected = PathBuf::from(MAIN_SEPARATOR_STR)
            .join("t")
            .join("f")
            .join("bar");

        assert_eq!(path.compact(), expected);
    }

    #[test]
    fn compacts_truncated_home_relative_paths() {
        let Some(home) = HOME_DIR.as_ref() else {
            return;
        };

        let path = home.join("Code").join("foo").join("bar");
        let expected = PathBuf::from("~").join("C").join("f").join("bar");

        assert_eq!(path.truncated().compact(), expected);
    }

    #[test]
    fn compacts_hidden_intermediate_components_without_dropping_them() {
        let path = PathBuf::from("~").join(".config").join("nvim");
        let expected = PathBuf::from("~").join(".c").join("nvim");

        assert_eq!(path.compact(), expected);
    }

    #[test]
    fn leaves_non_home_paths_unchanged() {
        let path = PathBuf::from(MAIN_SEPARATOR_STR).join("tmp").join("repo");

        assert_eq!(path.truncated(), path);
    }

    #[test]
    fn splits_compact_display_path_into_prefix_and_basename() {
        let Some(home) = HOME_DIR.as_ref() else {
            return;
        };

        let path = home
            .join("Code")
            .join("foo")
            .join("bar")
            .truncated()
            .compact();
        assert_eq!(
            path.split_last(),
            (
                PathBuf::from("~").join("C").join("f").as_path(),
                OsStr::new("bar")
            )
        );
    }

    #[test]
    fn splits_empty_path_into_empty_parent_and_basename() {
        let path = PathBuf::from("");

        assert_eq!(path.split_last(), (Path::new(""), OsStr::new("")));
    }

    #[test]
    fn splits_parent_dir_as_basename() {
        let path = PathBuf::from("foo").join("..");

        assert_eq!(
            path.split_last(),
            (PathBuf::from("foo").as_path(), OsStr::new(".."))
        );
    }

    #[test]
    fn splits_root_path_into_empty_parent_and_root_basename() {
        let path = PathBuf::from(MAIN_SEPARATOR_STR);

        assert_eq!(
            path.split_last(),
            (Path::new(""), OsStr::new(MAIN_SEPARATOR_STR))
        );
    }

    #[test]
    fn truncates_paths_under_cached_home() {
        let Some(home) = HOME_DIR.as_ref() else {
            return;
        };

        let path = home.join("repo");
        assert_eq!(path.truncated(), PathBuf::from("~").join("repo"));
    }
}
