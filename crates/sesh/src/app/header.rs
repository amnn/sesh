// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Rendering for the header bar.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Stylize as _;
use ratatui::text::Line;
use ratatui::text::Span;

use crate::app::highlight::Highlight;
use crate::app::span::push_repo_path_spans;
use crate::app::span::push_shortcut_span;
use crate::model::session::Repo;
use crate::model::session::Session;

/// Header bar component showing counts, repo context, and available actions.
pub(super) struct Header<'r> {
    confirm_delete: bool,
    found: usize,
    repo: Option<&'r Repo>,
    selected: Option<&'r Session>,
    total: usize,
}

impl<'r> Header<'r> {
    /// Create a header from the current picker state.
    pub(super) fn new(
        confirm_delete: bool,
        found: usize,
        repo: Option<&'r Repo>,
        selected: Option<&'r Session>,
        total: usize,
    ) -> Self {
        Self {
            confirm_delete,
            found,
            repo,
            selected,
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
        } else if self.selected.is_some_and(|s| s.can_delete()) {
            line += Span::raw(prefix).dim();
            push_shortcut_span(&mut line, "C-d");
            line += Span::raw(" delete");
            prefix = ", ";
        }

        if self.selected.is_some_and(|s| s.can_close()) {
            line += Span::raw(prefix).dim();
            push_shortcut_span(&mut line, "C-x");
            line += Span::raw(" close");
            prefix = ", ";
        }

        if let Some(flag) = self.selected.and_then(|s| s.flag()) {
            line += Span::raw(prefix).dim();
            push_shortcut_span(&mut line, "C-f");
            line += Span::raw(if flag { " unflag" } else { " flag" });
        }

        f.render_widget(line, area)
    }
}
