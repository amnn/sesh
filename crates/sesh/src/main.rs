// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! CLI entrypoint for `sesh`.

use std::collections::BTreeSet;
use std::env;
use std::path::Path;

use anyhow::Context as _;
use clap::Parser;
use clap::Subcommand;

use sesh::App;
use sesh::Session;
use sesh::jj;
use sesh::tmux;

#[derive(Debug, Parser)]
#[command(name = "sesh", version, about)]
#[command(arg_required_else_help = true)]
#[command(disable_help_flag = true)]
struct Args {
    #[arg(short = '?', long = "help", action = clap::ArgAction::Help, global = true)]
    help: Option<bool>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run `sesh` in a tmux pop-up.
    Popup {
        /// Width of the popup, passed to tmux as-is.
        #[arg(long = "width", default_value = "80%")]
        width: String,

        /// Height of the popup, passed to tmux as-is.
        #[arg(long = "height", default_value = "80%")]
        height: String,

        /// Title of the popup, passed to tmux as-is.
        #[arg(short = 'T', long = "title", default_value = "sesh")]
        title: String,

        /// Remaining arguments are forwarded to the underlying invocation of `sesh cli`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Run `sesh` in the current terminal.
    Cli {
        /// Additional repository globs to surface alongside existing tmux sessions.
        #[arg(short = 'r', long = "repo", value_name = "GLOB", action = clap::ArgAction::Append)]
        repos: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Args::parse().command {
        Command::Popup {
            width,
            height,
            title,
            args,
        } => tmux::popup(&width, &height, &title, &args),

        Command::Cli { repos } => {
            jj::ensure()?;
            tmux::ensure()?;

            let cwd = env::current_dir().context("failed to resolve current working directory")?;
            let current_repo = jj::repo_root(&cwd);

            let mut sessions = vec![];
            let mut seen_repos = BTreeSet::new();
            let mut seen_names = BTreeSet::new();

            // Add all the live sessions from tmux.
            for (name, repo) in tmux::sessions().await? {
                seen_names.insert(name.clone());
                seen_repos.extend(repo.clone());
                sessions.push(Session::from_tmux(name, repo))
            }

            // Add an entry for every repo found, as long as it's not already associated with a
            // live tmux session.
            for repo in jj::repos(&repos)? {
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
            let Some(session) = app.run()? else {
                return Ok(());
            };

            prepare_session(&session, &cwd).await?;
            tmux::switch_client(&session.name()).await?;
            Ok(())
        }
    }
}

/// Ensure the tmux session we are switching to is ready.
async fn prepare_session(session: &Session, cwd: &Path) -> anyhow::Result<()> {
    if session.is_tmux() {
        return Ok(());
    }

    let target = session.name();
    let cwd = session.repo().unwrap_or(cwd);
    tmux::new_session(&target, cwd).await?;

    let Some(repo) = session.repo() else {
        return Ok(());
    };

    tmux::set_option(&target, "@sesh.repo", repo).await?;
    Ok(())
}
