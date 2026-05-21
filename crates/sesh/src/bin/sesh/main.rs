// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! CLI entrypoint for `sesh`.

mod help;

use std::env;
use std::path::PathBuf;

use anyhow::Context as _;
use clap::ArgAction;
use clap::CommandFactory as _;
use clap::Parser;

use sesh::App;
use sesh::Context;
use sesh::config::SeshConfig;
use sesh::jj;
use sesh::tmux;

#[derive(Debug, Parser)]
#[command(name = "sesh", version, about, styles = help::STYLES)]
#[command(disable_help_flag = true)]
struct Args {
    /// Print brief help.
    #[arg(short = 'h', action = ArgAction::SetTrue)]
    help: bool,

    /// Print complete help.
    #[arg(long = "help", action = ArgAction::SetTrue)]
    long_help: bool,

    /// Path to a custom config file.
    #[arg(
        long,
        value_name = "PATH",
        long_help = "Path to a custom config file. When omitted, sesh reads \
                     $XDG_CONFIG_HOME/sesh/sesh.toml, or ~/.config/sesh/sesh.toml when \
                     $XDG_CONFIG_HOME is unset."
    )]
    config: Option<PathBuf>,

    /// Additional repository globs to surface alongside existing tmux sessions.
    #[arg(
        short = 'r',
        long = "repo",
        value_name = "GLOB",
        action = ArgAction::Append,
        long_help = "Additional repository globs to surface alongside existing tmux sessions. \
                     Pass once per glob; each matching jj repo can be used as context for new \
                     repo-backed workspaces."
    )]
    repos: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.long_help {
        help::write_long_help::<Args>()?;
        return Ok(());
    }
    if args.help {
        Args::command().print_help()?;
        return Ok(());
    }

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
