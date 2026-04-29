// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Helpers for querying and invoking tmux.

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::os::unix::process::CommandExt as _;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::ensure;
use shlex::Quoter;
use tokio::process::Command;
use which::which;

/// Validate that `tmux` is available on `$PATH`.
pub fn ensure() -> anyhow::Result<()> {
    ensure!(which("tmux").is_ok(), "'tmux' not found in PATH");
    Ok(())
}

/// Kill an existing tmux session.
pub async fn kill_session(session: &str) -> anyhow::Result<()> {
    let output = Command::new("tmux")
        .args(["kill-session", "-t", session])
        .output()
        .await
        .context("failed to kill tmux session")?;

    ensure!(
        output.status.success(),
        "error running 'tmux kill-session': {}",
        String::from_utf8_lossy(&output.stderr),
    );

    Ok(())
}

/// Create a detached tmux session.
pub async fn new_session(session: &str, cwd: &Path) -> anyhow::Result<()> {
    let output = Command::new("tmux")
        .args(["new-session", "-d", "-s", session, "-c"])
        .arg(cwd)
        .output()
        .await
        .context("failed to create tmux session")?;

    ensure!(
        output.status.success(),
        "error running 'tmux new-session': {}",
        String::from_utf8_lossy(&output.stderr),
    );

    Ok(())
}

/// Open sesh in a tmux popup and forward arguments to the `cli` command.
pub fn popup(width: &str, height: &str, title: &str, args: &[String]) -> anyhow::Result<()> {
    ensure!(env::var_os("TMUX").is_some(), "popups must run inside tmux");
    let exe = env::current_exe().context("failed to resolve current executable")?;

    let quoter = Quoter::new();
    let mut popup_cmd = format!(
        "{} cli",
        quoter
            .quote(exe.to_string_lossy().as_ref())
            .context("failed to quote executable path")?
    );

    for arg in args {
        popup_cmd.push(' ');
        popup_cmd.push_str(&quoter.quote(arg).context("failed to quote CLI argument")?);
    }

    let error = std::process::Command::new("tmux")
        .args([
            "display-popup",
            "-E",
            "-w",
            width,
            "-h",
            height,
            "-T",
            title,
            &popup_cmd,
        ])
        .exec();

    Err(error).context("failed to display popup")
}

/// Run a shell script in the context of a target pane.
pub async fn run_shell(target: &str, cwd: &Path, script: &str) -> anyhow::Result<()> {
    let output = Command::new("tmux")
        .args(["run-shell", "-t", target, "-c"])
        .arg(cwd)
        .arg(script)
        .output()
        .await
        .context("failed to run tmux shell command")?;

    ensure!(
        output.status.success(),
        "error running 'tmux run-shell': {}",
        String::from_utf8_lossy(&output.stderr),
    );

    Ok(())
}

/// Query tmux for current sessions and attached sesh repo metadata.
pub async fn sessions() -> anyhow::Result<BTreeMap<String, Option<PathBuf>>> {
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}\t#{@sesh.repo}"])
        .output()
        .await
        .context("failed to discover information on tmux sessions")?;

    ensure!(
        output.status.success(),
        "error running 'tmux list-sessions': {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let mut sessions = BTreeMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some((session, repo)) = line.split_once('\t') else {
            continue;
        };

        let session = session.trim();
        if session.is_empty() {
            continue;
        }

        let repo = repo.trim();
        let repo = if repo.is_empty() {
            None
        } else {
            Some(PathBuf::from(repo))
        };

        sessions.insert(session.to_owned(), repo);
    }

    Ok(sessions)
}

/// Set a tmux session option.
pub async fn set_option<V: AsRef<OsStr> + ?Sized>(
    session: &str,
    option: &str,
    value: &V,
) -> anyhow::Result<()> {
    let output = Command::new("tmux")
        .args(["set-option", "-t", session, option])
        .arg(value)
        .output()
        .await
        .context("failed to set tmux session option")?;

    ensure!(
        output.status.success(),
        "error running 'tmux set-option': {}",
        String::from_utf8_lossy(&output.stderr),
    );

    Ok(())
}

/// Switch the current tmux client to an existing session.
pub async fn switch_client(session: &str) -> anyhow::Result<()> {
    let output = Command::new("tmux")
        .args(["switch-client", "-t", session])
        .output()
        .await
        .context("failed to switch tmux client")?;

    ensure!(
        output.status.success(),
        "error running 'tmux switch-client': {}",
        String::from_utf8_lossy(&output.stderr),
    );

    Ok(())
}
