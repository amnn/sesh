// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Helpers for querying and invoking tmux.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::ensure;
use tokio::process::Command;
use which::which;

/// Metadata for a live tmux session.
#[derive(Debug)]
pub struct SessionInfo {
    /// Windows in the session that have an active bell alert.
    pub alerts: Vec<String>,

    /// Optional jj repository attached to the session.
    pub repo: Option<PathBuf>,
}

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

/// Query tmux for current sessions, attached sesh repo metadata, and bell alerts.
pub async fn sessions() -> anyhow::Result<BTreeMap<String, SessionInfo>> {
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

        sessions.insert(
            session.to_owned(),
            SessionInfo {
                alerts: vec![],
                repo,
            },
        );
    }

    let output = Command::new("tmux")
        .args([
            "list-windows",
            "-a",
            "-F",
            "#{session_name}\t#{window_index}\t#{window_bell_flag}",
        ])
        .output()
        .await
        .context("failed to discover tmux bell alerts")?;

    ensure!(
        output.status.success(),
        "error running 'tmux list-windows': {}",
        String::from_utf8_lossy(&output.stderr),
    );

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let fields: Vec<_> = line.splitn(3, '\t').collect();
        let [session, window, bell] = fields[..] else {
            continue;
        };

        if bell.trim() != "1" {
            continue;
        }

        if let Some(info) = sessions.get_mut(session.trim()) {
            info.alerts.push(window.trim().to_owned());
        }
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
