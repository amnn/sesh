// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! CLI entrypoint for `sesh`.

use std::env;
use std::path::PathBuf;

use anyhow::Context as _;
use clap::Parser;

use sesh::App;
use sesh::Context;
use sesh::config::SeshConfig;
use sesh::jj;
use sesh::tmux;

#[derive(Debug, Parser)]
#[command(name = "sesh", version, about)]
#[command(disable_help_flag = true)]
struct Args {
    #[arg(short = '?', long = "help", action = clap::ArgAction::Help)]
    help: Option<bool>,

    /// Path to a custom config file.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Additional repository globs to surface alongside existing tmux sessions.
    #[arg(short = 'r', long = "repo", value_name = "GLOB", action = clap::ArgAction::Append)]
    repos: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = SeshConfig::load(args.config.as_deref())?;

    jj::ensure()?;
    tmux::ensure()?;

    let cwd = env::current_dir().context("failed to resolve current working directory")?;
    let repo = jj::repo_root(&cwd);

    let context = Context {
        globs: &args.repos,
        setup: &config.tmux.setup,
    };

    App::new(repo).run(&cwd, context).await
}
