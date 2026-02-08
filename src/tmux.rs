use std::collections::BTreeMap;
use std::env;
use std::os::unix::process::CommandExt as _;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::ensure;
use shlex::Quoter;
use tokio::process::Command;
use which::which;

#[derive(Default)]
pub struct Meta {
    pub panes: Vec<String>,
    pub repo: Option<PathBuf>,
}

/// Validate that `tmux` is available on `$PATH`.
pub fn ensure() -> anyhow::Result<()> {
    ensure!(which("tmux").is_ok(), "'tmux' not found in PATH");
    Ok(())
}

/// Return the visible contents of a tmux pane, by its ID.
pub(crate) async fn pane(pane_id: &str) -> anyhow::Result<Vec<String>> {
    let output = Command::new("tmux")
        .args(["capture-pane", "-ep", "-t", pane_id])
        .output()
        .await
        .with_context(|| format!("failed to run 'tmux capture-pane' for pane '{pane_id}'"))?;

    ensure!(
        output.status.success(),
        "error running 'tmux capture-pane' for pane '{pane_id}': {}",
        String::from_utf8_lossy(&output.stderr),
    );

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect())
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

/// Query tmux for current sessions, their panes, and attached sesh repo metadata.
pub async fn sessions() -> anyhow::Result<BTreeMap<String, Meta>> {
    let metadata = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}\t#{@sesh.repo}"])
        .output();
    let panes = Command::new("tmux")
        .args(["list-panes", "-a", "-F", "#{session_name}\t#{pane_id}"])
        .output();

    let (metadata, panes) = tokio::try_join!(metadata, panes)
        .context("failed to discover information on tmux sessions")?;

    ensure!(
        metadata.status.success(),
        "error running 'tmux list-sessions': {}",
        String::from_utf8_lossy(&metadata.stderr),
    );

    ensure!(
        panes.status.success(),
        "error running 'tmux list-panes': {}",
        String::from_utf8_lossy(&panes.stderr),
    );

    let mut sessions = BTreeMap::new();
    for line in String::from_utf8_lossy(&metadata.stdout).lines() {
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
            Meta {
                panes: Vec::new(),
                repo,
            },
        );
    }

    for line in String::from_utf8_lossy(&panes.stdout).lines() {
        let Some((session, pane)) = line.split_once('\t') else {
            continue;
        };

        let session = session.trim();
        let pane = pane.trim();
        if session.is_empty() || pane.is_empty() {
            continue;
        }

        sessions
            .entry(session.to_owned())
            .or_default()
            .panes
            .push(pane.to_owned());
    }

    Ok(sessions)
}
