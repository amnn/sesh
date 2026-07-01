// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! CLI entrypoint for `sesh`.

mod help;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context as _;
use clap::ArgAction;
use clap::CommandFactory as _;
use clap::Parser as _;

use sesh::App;
use sesh::Context;
use sesh::Model;
use sesh::cmd::jj;
use sesh::cmd::tmux;
use sesh::config::SeshConfig;

#[derive(Debug, clap::Parser)]
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

    /// Seed the initial query.
    #[arg(short = 'q', long, value_name = "STR")]
    query: Option<String>,

    /// Automatically switch when the initial query has only one match.
    #[arg(short = '1', long = "select-1", action = ArgAction::SetTrue)]
    select_1: bool,

    /// Exit without opening the UI if the initial query has no matches.
    #[arg(short = '0', long = "exit-0", action = ArgAction::SetTrue)]
    exit_0: bool,

    /// Filter non-interactively using the initial query.
    #[arg(short = 'f', long, action = ArgAction::SetTrue)]
    filter: bool,

    /// Additional repository globs to surface alongside existing tmux sessions.
    #[arg(
        short = 'r',
        long = "repo",
        value_name = "GLOB",
        action = ArgAction::Append,
        long_help = "Additional repository globs to surface alongside existing tmux sessions. \
                     Pass once per glob; these stack with repo.globs from config, and each \
                     matching jj repo can be used as context for new repo-backed workspaces."
    )]
    repos: Vec<String>,
}

/// Parse CLI arguments, load config, and run the picker.
#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    let args = Args::parse();

    if args.long_help {
        help::write_long_help::<Args>()?;
        return Ok(ExitCode::SUCCESS);
    }

    if args.help {
        Args::command().print_help()?;
        return Ok(ExitCode::SUCCESS);
    }

    let config = SeshConfig::load(args.config.as_deref())?;
    let mut globs = config.repo.globs.clone();
    globs.extend(args.repos);

    jj::ensure()?;
    tmux::ensure()?;

    let cwd = env::current_dir().context("failed to resolve current working directory")?;
    let repo = jj::repo_root(&cwd);

    let query = args.query.unwrap_or_default();
    let mut model = Model::new(&globs, repo.as_deref(), query).await?;
    let matches = model.matches();

    if args.exit_0 && matches.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }

    if args.select_1
        && let [session] = &matches[..]
    {
        session.switch(&cwd, &config.tmux.setup).await?;
        return Ok(ExitCode::SUCCESS);
    }

    if args.filter {
        for session in &matches {
            println!("{}", session.name());
        }

        return Ok(if matches.is_empty() {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        });
    }

    let context = Context {
        globs: &globs,
        setup: &config.tmux.setup,
        sigil: config.ui.sigil,
    };

    App::new(repo, model).run(&cwd, context).await?;
    Ok(ExitCode::SUCCESS)
}
