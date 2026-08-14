// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! CLI entrypoint for `smth`.

mod agent;
mod help;

use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context as _;
use anyhow::bail;
use anyhow::ensure;
use clap::ArgAction;
use clap::ArgGroup;
use clap::CommandFactory as _;
use clap::Parser as _;

use smth::App;
use smth::Context;
use smth::Model;
use smth::cmd::jj;
use smth::cmd::tmux;
use smth::config::SmthConfig;

#[derive(Debug, clap::Parser)]
#[command(
    name = "smth",
    version,
    about = "switch to something else",
    styles = help::STYLES
)]
#[command(disable_help_flag = true)]
#[command(group(
    ArgGroup::new("action")
        .args(["filter", "json", "flag", "unflag", "create"])
        .multiple(false)
))]
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
        long_help = "Path to a custom config file. When omitted, smth reads \
                     $XDG_CONFIG_HOME/smth/smth.toml, or ~/.config/smth/smth.toml when \
                     $XDG_CONFIG_HOME is unset."
    )]
    config: Option<PathBuf>,

    /// Repository or workspace to use as the base context.
    #[arg(
        short = 'b',
        long,
        value_name = "REPO",
        conflicts_with = "no_base",
        long_help = "Repository or workspace to use as the base context. Named workspace paths use \
                     their default workspace when it is available, matching current-directory \
                     inference. When omitted, smth infers the base from the current working \
                     directory."
    )]
    base: Option<PathBuf>,

    /// Force an empty repository context.
    #[arg(
        short = 'B',
        long = "no-base",
        action = ArgAction::SetTrue,
        long_help = "Force an empty repository context instead of inferring one from the current \
                     working directory."
    )]
    no_base: bool,

    /// Revision used as the base for newly created workspaces.
    #[arg(
        short = 'o',
        long,
        value_name = "REV",
        long_help = "Revision used as the base for newly created workspaces. Defaults to trunk(). \
                     An explicit revision requires a repository base."
    )]
    onto: Option<String>,

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

    /// Print structured information about discovered entries.
    #[arg(
        long,
        action = ArgAction::SetTrue,
        conflicts_with_all = ["filter", "select_1"],
        long_help = "Print structured JSON information about discovered live sessions and \
                     repository candidates. The optional query narrows records using the same \
                     fuzzy matcher as the picker."
    )]
    json: bool,

    /// Mark a live session as flagged.
    #[arg(
        long,
        value_name = "SESSION",
        num_args = 0..=1,
        conflicts_with_all = ["query", "select_1", "exit_0"],
        long_help = "Mark a live session as flagged. The optional session name overrides a named \
                     workspace inferred from --base. The target must match the selected repository \
                     family and workspace identity."
    )]
    flag: Option<Option<String>>,

    /// Clear a live session's flag.
    #[arg(
        long,
        value_name = "SESSION",
        num_args = 0..=1,
        conflicts_with_all = ["query", "select_1", "exit_0"],
        long_help = "Clear a live session's flag. The optional session name overrides a named \
                     workspace inferred from --base. The target must match the selected repository \
                     family and workspace identity."
    )]
    unflag: Option<Option<String>>,

    /// Ensure a session exists without switching to it.
    #[arg(
        short = 'c',
        long,
        value_name = "SESSION",
        num_args = 0..=1,
        conflicts_with_all = ["query", "select_1", "exit_0"],
        long_help = "Ensure a session exists without switching to it. Existing repository \
                     checkouts receive a tmux session; missing named workspaces are created at \
                     --onto; plain sessions are created in the process working directory. Prints \
                     the actual tmux name."
    )]
    create: Option<Option<String>>,

    /// Additional repository globs to surface alongside existing tmux sessions.
    #[arg(
        short = 'r',
        long = "repo",
        value_name = "GLOB",
        action = ArgAction::Append,
        long_help = "Additional repository globs to surface alongside existing tmux sessions. \
                     Pass once per glob; these stack with repo.globs from config, and each \
                     matching jj repo can be used as context for new repo-backed workspaces. A \
                     leading ~ path component expands to the user's home directory."
    )]
    repos: Vec<String>,

    /// Operation to run instead of opening the picker.
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Publish agent lifecycle state on the current tmux pane.
    Agent(agent::Args),
}

/// Non-interactive root operation selected after parsing flat CLI options.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    /// Print fuzzy matches as names.
    Filter,

    /// Print fuzzy matches as structured records.
    Json,

    /// Set the target live session's manual flag.
    Flag(Option<String>),

    /// Clear the target live session's manual flag.
    Unflag(Option<String>),

    /// Ensure the target session exists without switching to it.
    Create(Option<String>),
}

impl Args {
    /// Return the selected non-interactive root action.
    #[allow(clippy::manual_map)]
    fn action(&self) -> Option<Action> {
        if self.filter {
            Some(Action::Filter)
        } else if self.json {
            Some(Action::Json)
        } else if let Some(session) = &self.flag {
            Some(Action::Flag(session.clone()))
        } else if let Some(session) = &self.unflag {
            Some(Action::Unflag(session.clone()))
        } else if let Some(session) = &self.create {
            Some(Action::Create(session.clone()))
        } else {
            None
        }
    }

    /// The base repository for the current smth invocation.
    ///
    /// Controlled by the `--base` and `--no-base` flags, or inferred from the current working
    /// directory. If `--base` is supplied, it must be a path inside a jj repo. If `--no-base` is
    /// supplied, the base is empty even if the current working directory is inside a jj repo.
    /// Otherwise, a base is set if the current working directory is inside a jj repo.
    fn base(&self, cwd: &Path) -> anyhow::Result<Option<PathBuf>> {
        if self.no_base {
            return Ok(None);
        }

        let Some(base) = &self.base else {
            return Ok(jj::repo_root(cwd));
        };

        let canonical = base
            .canonicalize()
            .with_context(|| format!("failed to normalize base '{}'", base.display()))?;

        let Some(root) = jj::repo_root(&canonical) else {
            bail!("--base '{}' is not inside a jj repo", base.display());
        };

        Ok(Some(root))
    }
}

/// Parse CLI arguments and run the requested command or picker.
#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    let args = Args::parse();
    let action = args.action();

    if args.long_help {
        help::write_long_help::<Args>()?;
        return Ok(ExitCode::SUCCESS);
    }

    if args.help {
        Args::command().print_help()?;
        return Ok(ExitCode::SUCCESS);
    }

    let config = SmthConfig::load(args.config.as_deref())?;

    if let Some(Command::Agent(args)) = args.command {
        args.run(&config.notification).await?;
        return Ok(ExitCode::SUCCESS);
    }

    jj::ensure()?;
    tmux::ensure()?;

    let cwd = env::current_dir().context("failed to resolve current working directory")?;
    let current = args.base(&cwd)?;

    ensure!(
        args.onto.is_none() || current.is_some(),
        "--onto requires a base repository",
    );

    let mut globs = config.repo.globs.clone();
    globs.extend(args.repos);

    let query = args.query.unwrap_or_default();
    let mut model = Model::new(&globs, current.as_deref(), query).await?;

    if action == Some(Action::Json) {
        let sessions = model.matches_json();
        println!("{}", serde_json::to_string_pretty(&sessions)?);
        return Ok(ExitCode::SUCCESS);
    }

    if let Some((session, flagged)) = match &action {
        Some(Action::Flag(session)) => Some((session.as_deref(), true)),
        Some(Action::Unflag(session)) => Some((session.as_deref(), false)),
        Some(Action::Filter | Action::Json | Action::Create(_)) | None => None,
    } {
        let session = model
            .session(current.as_deref(), session)
            .context("session not found")?;

        ensure!(session.is_live(), "session is not live");
        session.set_flag(flagged).await?;
        return Ok(ExitCode::SUCCESS);
    }

    if let Some(Action::Create(name)) = &action {
        let revision = args.onto.as_deref().unwrap_or(jj::DEFAULT_BASE_REVSET);
        let session = model.session_for_request(current.as_deref(), name.as_deref(), revision)?;
        let name = session.name();
        session.create(&cwd, &config.tmux.setup).await?;
        println!("{name}");
        return Ok(ExitCode::SUCCESS);
    }

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

    if action == Some(Action::Filter) {
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

    App::new(current, args.onto, model)
        .run(&cwd, context)
        .await?;
    Ok(ExitCode::SUCCESS)
}
