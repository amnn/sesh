// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! CLI entrypoint for `sesh`.

use std::collections::BTreeSet;
use std::env;

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

            let current_repo = env::current_dir().ok().and_then(|cwd| jj::repo_root(&cwd));

            let mut sessions = vec![];
            let mut seen = BTreeSet::new();

            for (name, repo) in tmux::sessions().await? {
                if let Some(repo) = &repo {
                    seen.insert(repo.clone());
                }

                sessions.push(Session::from_tmux(name, repo))
            }

            for repo in jj::repos(&repos)? {
                let inserted = seen.insert(repo.clone());
                if inserted {
                    sessions.push(Session::from_repo(repo)?);
                }
            }

            let app = App::new(sessions, current_repo);
            app.run()?;

            Ok(())
        }
    }
}
