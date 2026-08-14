// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Session serialization schema for non-interactive inspection.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::Serialize;

/// Serialized metadata for one live session or repository checkout candidate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SerializedSession {
    /// Normalized default-workspace path used to target this repository family; omitted when
    /// absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<PathBuf>,

    /// Plain tmux session or named workspace; omitted for a default workspace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Existing checkout attached to this entry; omitted when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,

    /// Actual tmux name used by a live session or reserved for this candidate.
    pub tmux: String,

    /// Whether the tmux session currently exists.
    pub live: bool,

    /// Whether this entry represents a named workspace that can be deleted.
    pub deletable: bool,

    /// Manual flag state for live sessions; omitted for non-live candidates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flagged: Option<bool>,

    /// Tmux windows whose bell or agent lifecycle state currently requires attention.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub attention: BTreeSet<String>,

    /// Agent lifecycle state counts published by panes; omitted when none are published.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub agents: BTreeMap<String, usize>,
}
