// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! A rendering wrapper for `Path` that truncates the home directory to `~`.

use std::env;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;

use once_cell::sync::Lazy;

static HOME_DIR: Lazy<Option<PathBuf>> =
    Lazy::new(|| env::home_dir().map(|home| home.canonicalize().unwrap_or(home)));

pub(crate) trait TruncatedExt {
    fn truncated(&self) -> TruncatedPath<'_>;
}

pub(crate) struct TruncatedPath<'p>(&'p Path);

impl TruncatedExt for Path {
    fn truncated(&self) -> TruncatedPath<'_> {
        TruncatedPath(self)
    }
}

impl TruncatedExt for PathBuf {
    fn truncated(&self) -> TruncatedPath<'_> {
        self.as_path().truncated()
    }
}

impl fmt::Display for TruncatedPath<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(path) = self;
        let Some(home) = HOME_DIR.as_deref() else {
            return path.display().fmt(f);
        };

        if let Ok(path) = path.strip_prefix(home) {
            write!(f, "~/{}", path.display())
        } else {
            path.display().fmt(f)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::TruncatedExt as _;

    #[test]
    fn leaves_non_home_paths_unchanged() {
        let path = PathBuf::from("/tmp/repo");

        assert_eq!(path.truncated().to_string(), "/tmp/repo");
    }

    #[test]
    fn truncates_paths_under_cached_home() {
        let Some(home) = super::HOME_DIR.as_ref() else {
            return;
        };

        let path = home.join("repo");
        assert_eq!(path.truncated().to_string(), "~/repo");
    }
}
