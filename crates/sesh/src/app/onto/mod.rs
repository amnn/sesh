// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! State for `onto` revision selection mode.

mod picker;

use std::collections::BTreeSet;
use std::path::PathBuf;

use ansi_to_tui::IntoText as _;
use anyhow::Context as _;
use anyhow::bail;
use anyhow::ensure;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::app::component::loader;
use crate::app::component::loader::Loader;
use crate::app::onto::picker::Picker;
use crate::cmd::jj;

/// Template used to resolve a selected log row into semantic revision metadata.
const BASE_REVISION_TEMPLATE: &str = concat!(
    r#"change_id.short() ++ "\t" ++ self.contained_in("trunk()") ++ "\t" ++ "#,
    r#"local_bookmarks ++ "\t" ++ remote_bookmarks ++ "\n""#,
);

/// Result of handling a key while `onto` revision selection is active.
pub(super) enum Action {
    /// Accept the selected commit as the `onto` revision.
    Accept,

    /// Leave `onto` revision selection mode.
    Cancel,
}

/// Query, picker, and loading state for `onto` revision selection.
pub(super) struct State {
    picker: loader::State<Picker>,
    repo: PathBuf,
    state: picker::State,
}

/// Semantic revision metadata resolved from a selected commit.
#[derive(Debug, Eq, PartialEq)]
struct BaseRevisionMetadata {
    /// Short change ID for the selected commit.
    change_id: String,
    /// Whether the selected commit is the configured trunk revision.
    is_trunk: bool,
    /// Local bookmarks pointing to the selected commit.
    local_bookmarks: BTreeSet<String>,
    /// Remote bookmarks pointing to the selected commit.
    remote_bookmarks: BTreeSet<String>,
}

impl State {
    /// Create onto-selection state and start loading the current repo's log output.
    pub(super) fn new(repo: PathBuf) -> Self {
        let picker = loader::State::new({
            let repo = repo.clone();
            async move {
                let text = jj::log(&repo)
                    .await
                    .with_context(|| {
                        format!("failed to build onto picker for repo '{}'", repo.display())
                    })?
                    .into_bytes()
                    .into_text()
                    .context("failed to render jj log output")?;

                Ok(Picker::new(text))
            }
        });

        Self {
            picker,
            repo,
            state: picker::State::default(),
        }
    }

    /// Resolve the selected commit and return its preferred semantic revision.
    pub(super) async fn accept(&self) -> anyhow::Result<String> {
        let picker = self.picker.view().context("onto picker has not loaded")?;
        let revision = self
            .state
            .selected_revision(picker)
            .context("onto picker has no selected revision")?;

        let output = jj::show(&self.repo, revision, BASE_REVISION_TEMPLATE).await?;
        let metadata = BaseRevisionMetadata::parse(&output)?;
        Ok(metadata.preferred_revision().to_owned())
    }

    /// Render the onto picker into `area`.
    pub(super) fn draw(&mut self, f: &mut Frame<'_>, area: Rect) {
        f.render_stateful_widget(Loader::new(&mut self.state), area, &mut self.picker);

        if let Some(picker) = self.picker.pending() {
            self.state.initialize(picker);
            self.picker.finish();
        }
    }

    /// Handle a key event while `onto` revision selection mode is active.
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        use KeyCode as KC;
        use KeyModifiers as KM;

        const CTRL: KM = KM::CONTROL;
        const SHIFT: KM = KM::SHIFT;

        match key.code {
            // Accept the selected commit.
            KC::Enter => return Some(Action::Accept),

            // Cancel
            KC::Esc => return Some(Action::Cancel),
            KC::Char('c' | 'g' | 'o') if key.modifiers.contains(CTRL) => {
                return Some(Action::Cancel);
            }

            // Select commit
            KC::Up => self.state.select_previous(),
            KC::Down => self.state.select_next(),
            KC::Tab => self.state.select_next_match(),
            KC::BackTab => self.state.select_previous_match(),

            // Edit query
            KC::Backspace => self.state.model.pop(),
            KC::Char('u') if key.modifiers.contains(CTRL) => self.state.model.clear(),
            KC::Char(c) if key.modifiers.is_empty() => self.state.model.push(c),
            KC::Char(c) if key.modifiers.contains(SHIFT) => self.state.model.push(c),

            _ => {}
        }

        None
    }

    /// Return the current `onto` revision query.
    pub(super) fn query(&self) -> &str {
        self.state.model.query()
    }
}

impl BaseRevisionMetadata {
    /// Parse one tab-delimited metadata row emitted by `BASE_REVISION_TEMPLATE`.
    fn parse(output: &str) -> anyhow::Result<Self> {
        let mut lines = output.lines();
        let line = lines.next().context("missing base revision metadata")?;
        ensure!(
            lines.next().is_none(),
            "expected exactly one base revision metadata row"
        );

        let fields: Vec<_> = line.split('\t').collect();
        let [change_id, is_trunk, local_bookmarks, remote_bookmarks] = fields[..] else {
            bail!(
                "expected four base revision metadata fields, found {}",
                fields.len()
            );
        };

        ensure!(!change_id.is_empty(), "missing change ID");

        let change_id = change_id.to_owned();
        let is_trunk: bool = is_trunk.parse().context("invalid trunk membership")?;
        let local_bookmarks = local_bookmarks
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        let remote_bookmarks = remote_bookmarks
            .split_whitespace()
            .map(str::to_owned)
            .collect();

        Ok(Self {
            change_id,
            is_trunk,
            local_bookmarks,
            remote_bookmarks,
        })
    }

    /// Return the most stable semantic revision for this commit.
    fn preferred_revision(&self) -> &str {
        if self.is_trunk {
            jj::DEFAULT_BASE_REVSET
        } else if let Some(local) = self.local_bookmarks.first() {
            local
        } else if let Some(remote) = self.remote_bookmarks.first() {
            remote
        } else {
            &self.change_id
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(
        is_trunk: bool,
        local_bookmarks: &[&str],
        remote_bookmarks: &[&str],
    ) -> BaseRevisionMetadata {
        BaseRevisionMetadata {
            change_id: "change-id".to_owned(),
            is_trunk,
            local_bookmarks: local_bookmarks
                .iter()
                .map(|bookmark| (*bookmark).to_owned())
                .collect(),
            remote_bookmarks: remote_bookmarks
                .iter()
                .map(|bookmark| (*bookmark).to_owned())
                .collect(),
        }
    }

    #[test]
    fn parses_base_revision_metadata() {
        let metadata = BaseRevisionMetadata::parse(
            "change-id\tfalse\tlocal-one local-two\tremote-one remote-two\n",
        )
        .unwrap();

        assert_eq!(
            metadata,
            BaseRevisionMetadata {
                change_id: "change-id".to_owned(),
                is_trunk: false,
                local_bookmarks: ["local-one".to_owned(), "local-two".to_owned()].into(),
                remote_bookmarks: ["remote-one".to_owned(), "remote-two".to_owned()].into(),
            }
        );
    }

    #[test]
    fn preferred_revision_falls_back_to_change_id() {
        let metadata = metadata(false, &[], &[]);

        assert_eq!(metadata.preferred_revision(), "change-id");
    }

    #[test]
    fn preferred_revision_prefers_local_bookmark() {
        let metadata = metadata(false, &["local-one", "local-two"], &["remote"]);

        assert_eq!(metadata.preferred_revision(), "local-one");
    }

    #[test]
    fn preferred_revision_prefers_remote_bookmark() {
        let metadata = metadata(false, &[], &["remote-one", "remote-two"]);

        assert_eq!(metadata.preferred_revision(), "remote-one");
    }

    #[test]
    fn preferred_revision_prefers_trunk() {
        let metadata = metadata(true, &["local"], &["remote"]);

        assert_eq!(metadata.preferred_revision(), jj::DEFAULT_BASE_REVSET);
    }

    #[test]
    fn rejects_malformed_base_revision_metadata() {
        for output in [
            "",
            "change-id\ttrue\tlocal",
            "change-id\tmaybe\t\t",
            "change-id\ttrue\t\t\textra",
            "change-id\ttrue\t\t\nextra\tfalse\t\t",
            "\tfalse\t\t",
        ] {
            assert!(
                BaseRevisionMetadata::parse(output).is_err(),
                "unexpectedly parsed {output:?}"
            );
        }
    }
}
