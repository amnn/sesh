// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! User configuration loaded from `sesh.toml`.

use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use serde::Deserialize;
use serde::Serialize;

/// The relative config file path below the `sesh` config root.
pub const PATH: &str = "sesh.toml";

/// Top-level `sesh` config file schema.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct SeshConfig {
    /// Configuration for creating and initializing tmux sessions.
    pub tmux: TmuxConfig,
}

/// Configuration for creating and initializing tmux sessions.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct TmuxConfig {
    /// Shell script to run after creating a detached tmux session.
    pub setup: String,
}

impl SeshConfig {
    /// Load config from an explicit path, or from the default XDG config location.
    ///
    /// If no explicit path is supplied and the default config file is missing, returns the built-in
    /// default config. An explicit path must exist.
    pub fn load(path: Option<&Path>) -> anyhow::Result<Self> {
        let Some(contents) = read_to_string(path)? else {
            return Ok(Self::default());
        };

        toml::from_str(&contents).context("could not parse config")
    }
}

fn read_to_string(path: Option<&Path>) -> anyhow::Result<Option<String>> {
    if let Some(path) = path {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("could not read '{}'", path.display()))?;
        return Ok(Some(contents));
    }

    let root = if let Some(config) = env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(config)
    } else {
        let home = env::var_os("HOME").context("could not find $HOME directory")?;
        PathBuf::from(home).join(".config")
    };

    let path = root.join("sesh").join(PATH);
    match fs::read_to_string(&path) {
        Ok(contents) => Ok(Some(contents)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).with_context(|| format!("could not read '{}'", path.display())),
    }
}
