// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Helpers for markdown-driven `sesh` integration tests.

mod env;
mod parser;
mod runner;
mod svg;
mod tmux;

use std::fs;
use std::path::Path;

use anyhow::Context as _;

pub use crate::parser::Script;
pub use crate::runner::Runner;

/// Remove snapshot artifacts whose markdown case no longer exists.
pub fn remove_stale_snapshots(root: &Path) -> anyhow::Result<()> {
    let mut dirs = vec![root.to_owned()];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };

            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }

            let Some("snap" | "svg") = path.extension().and_then(|ext| ext.to_str()) else {
                continue;
            };

            let mut case = path.clone();
            while case.extension().is_some_and(|ext| ext != "snap") {
                case.set_extension("");
            }

            if case.extension().and_then(|ext| ext.to_str()) != Some("snap") {
                continue;
            }

            case.set_extension("");
            if !case.exists() {
                fs::remove_file(path).ok();
            }
        }
    }

    Ok(())
}

/// Remove stale SVG artifacts associated with `snapshot` before rewriting it.
pub fn remove_test_svg_snapshots(snapshot: &Path) -> anyhow::Result<()> {
    let Some(dir) = snapshot.parent() else {
        return Ok(());
    };

    let prefix = snapshot
        .file_name()
        .context("snapshot must have a file name")?
        .as_encoded_bytes();

    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };

    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };

        let path = entry.path();
        let Some(name) = path.file_name() else {
            continue;
        };

        if path.extension().and_then(|e| e.to_str()) != Some("svg") {
            continue;
        }

        if name.as_encoded_bytes().starts_with(prefix) {
            fs::remove_file(path).ok();
        }
    }

    Ok(())
}
