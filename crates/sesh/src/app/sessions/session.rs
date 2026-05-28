// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Rendering and preview prefetching for individual sessions.

use ansi_to_tui::IntoText as _;
use anyhow::Context as _;
use async_trait::async_trait;
use ratatui::style::Style;
use ratatui::style::Stylize as _;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use unicode_width::UnicodeWidthStr as _;

use crate::app::component::row::Row;
use crate::app::highlight::Highlight;
use crate::app::span::push_repo_path_spans;
use crate::cmd::jj;
use crate::model::prefetch;
use crate::model::session::DELIM_SUFFIX;
use crate::model::session::NAME_WIDTH;
use crate::model::session::Session;

const SIGIL_DELETE: &str = "×";
const SIGIL_TMUX: &str = "⬤";

#[async_trait]
impl prefetch::Key for Session {
    type Value = Text<'static>;

    /// Render a `jj log` preview for this session's attached repository.
    async fn fetch(&self) -> anyhow::Result<Self::Value> {
        let Some(repo) = self.preview_repo() else {
            return Ok(Text::raw(""));
        };

        jj::log(&repo)
            .await
            .with_context(|| format!("failed to build preview for repo '{}'", repo.display()))?
            .into_bytes()
            .into_text()
            .context("failed to render jj log output")
    }
}

/// Render `session` as a session-list row.
pub(super) fn row(session: &Session, highlighted: bool, deleting: bool, matches: &[u32]) -> Row {
    let mut hl = Highlight::new(matches.to_vec());
    let mut line = Line::default();
    push_session_name_spans(&mut line, session, &mut hl, deleting, highlighted);

    if let Some(repo) = session.repo() {
        let padding = NAME_WIDTH.saturating_sub(session.name().width()) + 1;
        line.extend(hl.highlight(Span::raw(" ".repeat(padding))));
        push_repo_path_spans(&mut line, &repo, &mut hl);
    };

    let row = Row::new(line);

    if highlighted && deleting {
        return row.with_sigil(Span::raw(SIGIL_DELETE).on_light_red());
    }

    let Some(flagged) = session.flag() else {
        return row;
    };

    let alerts = session.alerts();
    if !alerts.is_empty() && highlighted {
        row.with_sigil(Span::raw(SIGIL_TMUX).on_light_yellow())
    } else if !alerts.is_empty() {
        row.with_sigil(Span::raw(SIGIL_TMUX).light_yellow())
    } else if flagged && highlighted {
        row.with_sigil(Span::raw(SIGIL_TMUX).on_light_blue())
    } else if flagged {
        row.with_sigil(Span::raw(SIGIL_TMUX).light_blue())
    } else {
        row.with_sigil(Span::raw(SIGIL_TMUX).dim())
    }
}

/// Push styled session name spans, dimming a disambiguation suffix when present.
fn push_session_name_spans<'a>(
    spans: &mut impl Extend<Span<'a>>,
    session: &Session,
    hl: &mut Highlight,
    deleting: bool,
    highlighted: bool,
) {
    let name = session.name();

    let name_style = if deleting && highlighted {
        Style::new().on_light_red().bold()
    } else {
        Style::new()
    };

    let suffix_style = name_style.dim();

    let Some((prefix, suffix)) = name.rsplit_once(DELIM_SUFFIX) else {
        spans.extend(hl.highlight(Span::styled(name, name_style)));
        return;
    };

    spans.extend(hl.highlight(Span::styled(prefix.to_owned(), name_style)));
    spans.extend(hl.highlight(Span::styled(DELIM_SUFFIX, suffix_style)));
    spans.extend(hl.highlight(Span::styled(suffix.to_owned(), suffix_style)));
}
