// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! A rendering wrapper for `Path` that truncates the home directory to `~`.

use std::env;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;

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
        let Some(home) = env::home_dir() else {
            return path.display().fmt(f);
        };

        if let Ok(path) = path.strip_prefix(&home) {
            write!(f, "~/{}", path.display())
        } else {
            path.display().fmt(f)
        }
    }
}
