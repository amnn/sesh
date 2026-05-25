// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Rendering for the header bar.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Stylize as _;
use ratatui::text::Line;
use ratatui::text::Span;

use crate::session::Repo;
use crate::ui::Highlight;
use crate::ui::push_repo_path_spans;
use crate::ui::push_shortcut_span;

/// Header bar component showing counts, repo context, and available actions.
pub(super) struct Header<'r> {
    can_close: bool,
    can_delete: bool,
    confirm_delete: bool,
    found: usize,
    repo: Option<&'r Repo>,
    total: usize,
}

impl<'r> Header<'r> {
    /// Create a header from the current picker state.
    pub(super) fn new(
        can_close: bool,
        can_delete: bool,
        confirm_delete: bool,
        found: usize,
        repo: Option<&'r Repo>,
        total: usize,
    ) -> Self {
        Self {
            can_close,
            can_delete,
            confirm_delete,
            found,
            repo,
            total,
        }
    }

    /// Render the header bar into `area`.
    pub(super) fn draw(&self, f: &mut Frame<'_>, area: Rect) {
        let width = if self.total == 0 {
            1
        } else {
            self.total.ilog10() as usize + 1
        };

        let mut line = Line::default();

        line += Span::raw(format!(" {:>width$}", self.found));
        line += Span::raw(format!("/{} | ", self.total)).dim();
        push_shortcut_span(&mut line, "C-r");
        line += Span::raw(" repo: ");

        if let Some(repo) = self.repo {
            push_repo_path_spans(&mut line, repo.source(), &mut Highlight::none());
            line += Span::raw(", ").dim();
            push_shortcut_span(&mut line, "C-o");
            line += Span::raw(" onto: ");
            line += Span::raw(repo.revision().to_owned()).dim();
        } else {
            line += Span::raw("none").dim();
        }

        let mut prefix = " | ";
        if self.confirm_delete {
            line += Span::raw(prefix).dim();
            push_shortcut_span(&mut line, "C-y");
            line += Span::raw(" confirm").light_red().bold();
            prefix = ", ";
        } else if self.can_delete {
            line += Span::raw(prefix).dim();
            push_shortcut_span(&mut line, "C-d");
            line += Span::raw(" delete");
            prefix = ", ";
        }

        if self.can_close {
            line += Span::raw(prefix).dim();
            push_shortcut_span(&mut line, "C-x");
            line += Span::raw(" close");
        }

        f.render_widget(line, area)
    }
}
