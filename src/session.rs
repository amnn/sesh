use std::borrow::Cow;

use anyhow::Context as _;
use skim::prelude::ItemPreview;
use skim::prelude::PreviewContext;
use skim::prelude::SkimItem;

use crate::preview::preview;
use crate::tmux;

/// TODO: Replace with config
const PANE_HEIGHT: usize = 10;

/// A tmux session and its pane IDs.
#[derive(Clone, Debug)]
pub struct Session {
    name: String,
    pane_ids: Vec<String>,
}

impl Session {
    pub(crate) fn new(name: String) -> Self {
        Self {
            name,
            pane_ids: Vec::new(),
        }
    }

    pub(crate) fn add_pane_id(&mut self, pane_id: String) {
        self.pane_ids.push(pane_id);
    }

    /// Return the session name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Render a stacked pane preview for this session using current pane contents from `tmux`.
    pub fn preview(&self, width: usize) -> anyhow::Result<String> {
        let mut panes = Vec::with_capacity(self.pane_ids.len());

        for pane_id in &self.pane_ids {
            panes.push(tmux::pane(pane_id).with_context(|| {
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
