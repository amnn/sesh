// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Shared UI rendering helpers.

use std::path::MAIN_SEPARATOR;
use std::path::MAIN_SEPARATOR_STR;
use std::path::Path;

use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;

use crate::path::TruncatedExt as _;

/// Append a compact repository path, dimming the parent prefix and leaving the basename undimmed.
pub(crate) fn push_repo_path_spans(line: &mut Line<'_>, repo: &Path) {
    let repo = repo.truncated().compact();
    let (parent, base) = repo.split_last();

    let parent = parent.display().to_string();
    let separator = if !parent.is_empty() && !parent.ends_with(MAIN_SEPARATOR) {
        MAIN_SEPARATOR_STR
    } else {
        ""
    };

    let dim = Style::new().dim();
    *line += Span::styled(parent, dim);
    *line += Span::styled(separator, dim);
    *line += Span::raw(base.display().to_string());
}

/// Append a consistently styled shortcut token for header help text.
pub(crate) fn push_shortcut_span(line: &mut Line<'_>, code: &str) {
    let dim = Style::new().dim();
    let key = Style::new().fg(Color::Magenta);

    line.extend([
        Span::styled("[", dim),
        Span::styled(code.to_owned(), key),
        Span::styled("]", dim),
    ])
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn repo_path_keeps_relative_basename_unprefixed() {
        let mut line = Line::default();

        push_repo_path_spans(&mut line, Path::new("alpha"));

        assert_eq!(line.spans[0].content, "");
        assert_eq!(line.spans[1].content, "");
        assert_eq!(line.spans[2].content, "alpha");
    }

    #[test]
    fn repo_path_omits_duplicate_separator_after_root_parent() {
        let mut line = Line::default();

        push_repo_path_spans(&mut line, Path::new("/repo"));

        assert_eq!(line.spans[0].content, "/");
        assert_eq!(line.spans[1].content, "");
        assert_eq!(line.spans[2].content, "repo");
    }
}
