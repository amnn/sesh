// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Rendering for the header bar.

use std::path::Path;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::ui::push_repo_path_spans;
use crate::ui::push_shortcut_span;

pub(super) struct Header<'r> {
    can_close: bool,
    found: usize,
    repo: Option<&'r Path>,
    total: usize,
}

impl<'r> Header<'r> {
    pub(super) fn new(can_close: bool, found: usize, repo: Option<&'r Path>, total: usize) -> Self {
        Self {
            can_close,
            found,
            repo,
            total,
        }
    }

    pub(super) fn draw(&self, f: &mut Frame<'_>, area: Rect) {
        let width = if self.total == 0 {
            1
        } else {
            self.total.ilog10() as usize + 1
        };

        let mut line = Line::default();
        let dim = Style::new().dim();

        line += Span::raw(format!(" {:>width$}", self.found));
        line += Span::styled(format!("/{} | ", self.total), dim);
        push_shortcut_span(&mut line, "C-r");
        line += Span::raw(" repo: ");

        if let Some(repo) = self.repo {
            push_repo_path_spans(&mut line, repo);
        } else {
            line += Span::styled("none", dim);
        }

        if self.can_close {
            line += Span::styled(" | ", dim);
            push_shortcut_span(&mut line, "C-x");
            line += Span::raw(" close");
        }

        f.render_widget(line, area)
    }
}
