use std::borrow::Cow;
use std::env;
use std::path::PathBuf;

use anyhow::Context as _;
use futures::future;
use skim::prelude::ItemPreview;
use skim::prelude::PreviewContext;
use skim::prelude::SkimItem;

use crate::tmux;

/// TODO: Replace with config
const PANE_HEIGHT: usize = 10;

/// A tmux session and its pane IDs.
#[derive(Clone, Debug)]
pub struct Session {
    name: String,
    panes: Vec<String>,
    repo: Option<PathBuf>,
}

impl Session {
    /// Construct a potential session from information extracted from `tmux`.
    ///
    /// `name` is a tmux session name, `panes` is a list of tmux pane IDs, and `repo` is an
    /// optional path to a jj repository, that is attached as a user-option on the tmux session.
    pub fn from_tmux(name: String, panes: Vec<String>, repo: Option<PathBuf>) -> Self {
        Self { name, panes, repo }
    }

    /// Construct a potential session from a repository path.
    ///
    /// The session's name is derived from the repository's root directory name.
    pub fn from_repo(path: PathBuf) -> anyhow::Result<Self> {
        let name = path
            .file_name()
            .context("invalid repo: no directory name")?
            .to_string_lossy()
            .into_owned();

        Ok(Self {
            name,
            panes: vec![],
            repo: Some(path),
        })
    }

    /// Return the session name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Render a stacked pane preview for this session using current pane contents from `tmux`.
    pub fn preview(&self, width: usize) -> anyhow::Result<String> {
        let rt = tokio::runtime::Handle::current();
        let ids = self.panes.clone();

        // Fetch pane contents concurrently, re-using the running tokio runtime. This function is
        // not async but runs in the context of a tokio runtime.
        let panes = tokio::task::block_in_place(|| {
            rt.block_on(future::try_join_all(ids.into_iter().map(|id| async move {
                tmux::pane(&id, PANE_HEIGHT)
                    .await
                    .with_context(|| format!("failed to capture pane '{id}'"))
            })))
        })?;

        let mut prefix = "";
        let mut preview = String::new();
        for pane in panes {
            preview.push_str(prefix);
            prefix = "\n";

            preview.push_str(&pane);
            preview.push_str(prefix);
            preview.push_str(&"─".repeat(width));
        }

        Ok(preview)
    }
}

impl SkimItem for Session {
    fn text(&self) -> Cow<'_, str> {
        let Some(repo) = &self.repo else {
            return self.name().into();
        };

        if let Some(home) = env::home_dir()
            && let Ok(repo) = repo.strip_prefix(&home)
        {
            format!("{:<40} ~/{}", self.name(), repo.display()).into()
        } else {
            format!("{:<40} {}", self.name(), repo.display()).into()
        }
    }

    fn preview(&self, context: PreviewContext) -> ItemPreview {
        match self.preview(context.width) {
            Ok(preview) => ItemPreview::Text(preview),
            Err(error) => ItemPreview::Text(format!("Failed to render preview: {error:?}")),
        }
    }
}
