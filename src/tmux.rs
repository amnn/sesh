use std::collections::BTreeMap;
use std::env;
use std::os::unix::process::CommandExt as _;
use std::process::Command;

use anyhow::Context as _;
use anyhow::ensure;
use shlex::Quoter;
use which::which;

use crate::session::Session;

/// Validate that `tmux` is available on `$PATH`.
pub fn ensure() -> anyhow::Result<()> {
    ensure!(which("tmux").is_ok(), "'tmux' not found in PATH");
    Ok(())
}

/// Return the visible contents of a tmux pane, by its ID.
pub(crate) fn pane(pane_id: &str) -> anyhow::Result<Vec<String>> {
    let output = Command::new("tmux")
        .args(["capture-pane", "-ep", "-t", pane_id])
        .output()
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

    let error = Command::new("tmux")
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

/// Query `tmux` for the current sessions and their pane IDs.
pub fn sessions() -> anyhow::Result<Vec<Session>> {
    let output = std::process::Command::new("tmux")
        .args(["list-panes", "-a", "-F", "#{session_name}\t#{pane_id}"])
        .output()
        .context("failed to run 'tmux list-panes'")?;

    ensure!(
        output.status.success(),
        "error running 'tmux list-panes': {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let mut sessions = BTreeMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
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
            .or_insert_with(|| Session::new(session.to_owned()))
            .add_pane_id(pane.to_owned());
    }

    Ok(sessions.into_values().collect())
}
