use std::borrow::Cow;
use std::collections::BTreeMap;

use anyhow::Context as _;
use anyhow::ensure;
use skim::prelude::ItemPreview;
use skim::prelude::PreviewContext;
use skim::prelude::SkimItem;
use which::which;

use crate::preview::preview;

/// TODO: Replace with config
const PANE_HEIGHT: usize = 10;

/// A tmux session and its pane IDs.
#[derive(Clone, Debug)]
pub struct Session {
    name: String,
    pane_ids: Vec<String>,
}

impl Session {
    fn new(name: String) -> Self {
        Self {
            name,
            pane_ids: Vec::new(),
        }
    }

    /// Return the session name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Render a stacked pane preview for this session using current pane contents from `tmux`.
    pub fn preview(&self, width: usize) -> anyhow::Result<String> {
        let mut panes = Vec::with_capacity(self.pane_ids.len());

        for pane_id in &self.pane_ids {
            panes.push(pane_content(pane_id).with_context(|| {
                format!(
                    "failed to capture pane '{pane_id}' for session '{}'",
                    self.name()
                )
            })?);
        }

        Ok(preview(width, PANE_HEIGHT, panes.iter()))
    }
}

impl SkimItem for Session {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.name())
    }

    fn preview(&self, context: PreviewContext) -> ItemPreview {
        match self.preview(context.width) {
            Ok(preview) => ItemPreview::AnsiText(preview),
            Err(error) => ItemPreview::Text(format!("Failed to render preview: {error:?}")),
        }
    }
}

/// Validate that `tmux` is available on `$PATH`.
pub fn ensure() -> anyhow::Result<()> {
    ensure!(which("tmux").is_ok(), "'tmux' not found in PATH");
    Ok(())
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
            .pane_ids
            .push(pane.to_owned());
    }

    Ok(sessions.into_values().collect())
}

fn pane_content(pane_id: &str) -> anyhow::Result<Vec<String>> {
    let output = std::process::Command::new("tmux")
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
