// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! CLI entrypoint for `sesh`.

use std::collections::BTreeSet;
use std::env;
use std::path::PathBuf;

use anyhow::Context as _;
use clap::Parser;

use sesh::Action;
use sesh::App;
use sesh::Session;
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
    let current_repo = jj::repo_root(&cwd);

    let mut sessions = vec![];
    let mut seen_repos = BTreeSet::new();
    let mut seen_names = BTreeSet::new();

    // Add all the live sessions from tmux.
    for (name, info) in tmux::sessions().await? {
        seen_names.insert(name.clone());
        seen_repos.extend(info.repo.clone());
        sessions.push(Session::from_tmux(name, info.repo, info.alerts))
    }

    // Add an entry for every repo found, as long as it's not already associated with a
    // live tmux session.
    for repo in jj::repos(&args.repos)? {
        let inserted = seen_repos.insert(repo.clone());
        if !inserted {
            continue;
        }

        let mut session = Session::from_repo(repo)?;

        // Make sure the name that will be used for a new session associated with this repo
        // will be unambiguous by adding a suffix.
        let mut i = 1;
        while !seen_names.insert(session.name()) {
            session.set_suffix(i.to_string());
            i += 1;
        }

        sessions.push(session);
    }

    let app = App::new(sessions, current_repo);

    match app.run()? {
        Action::Cancel => Ok(()),
        Action::Close(session) => tmux::kill_session(&session.name()).await,
        Action::Switch(session) => session.switch(&cwd, &config.tmux.setup).await,
    }
}
