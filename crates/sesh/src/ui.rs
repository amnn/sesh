// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Shared UI rendering helpers.

use std::path::MAIN_SEPARATOR;
use std::path::MAIN_SEPARATOR_STR;
use std::path::Path;

use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Span;

use crate::path::TruncatedExt as _;

static HIGHLIGHT: Style = Style::new().blue().bold();

/// Overlay fuzzy match highlighting onto `spans`, preserving each span's existing style.
///
/// `indices` must be sorted character indices into the concatenated span text.
pub(crate) fn highlight(input: Vec<Span<'static>>, indices: &[u32]) -> Vec<Span<'static>> {
    if indices.is_empty() {
        return input;
    }

    let mut output = Vec::with_capacity(input.len());
    let mut is = indices.iter().copied().peekable();
    let mut cs = input
        .iter()
        .flat_map(|s| s.content.chars().map(|ch| (ch, s.style)))
        .enumerate()
        .peekable();

    loop {
        let Some(&(_, (_, base))) = cs.peek() else {
            break;
        };

        // Consume all the non-highlighted characters before the next highlighted index.
        let next = is.peek().copied().unwrap_or(u32::MAX);
        let mut content = String::new();
        while let Some((pos, (ch, style))) = cs.peek()
            && next > *pos as u32
            && style == &base
        {
            content.push(*ch);
            cs.next();
        }

        // If there is an unhighlighted prefix, push it and then restart the outer loop, because
        // the next character may still be unhighlighted but with a different style.
        if !content.is_empty() {
            output.push(Span::styled(content, base));
            continue;
        }

        // Gather the prefix of highlighted characters, overriding their style.
        let mut highlighted = String::new();
        while let (Some((pos, (ch, _))), Some(ix)) = (cs.peek(), is.peek())
            && *ix == *pos as u32
        {
            highlighted.push(*ch);
            cs.next();
            is.next();
        }

        if !highlighted.is_empty() {
            output.push(Span::styled(highlighted, HIGHLIGHT))
        }
    }

    output
}

/// Append a compact repository path, dimming the parent prefix and leaving the basename undimmed.
pub(crate) fn push_repo_path_spans<'a>(spans: &mut impl Extend<Span<'a>>, repo: &Path) {
    let repo = repo.truncated().compact();
    let (parent, base) = repo.split_last();

    let parent = parent.display().to_string();
    let separator = if !parent.is_empty() && !parent.ends_with(MAIN_SEPARATOR) {
        MAIN_SEPARATOR_STR
    } else {
        ""
    };

    let dim = Style::new().dim();
    spans.extend([
        Span::styled(parent, dim),
        Span::styled(separator, dim),
        Span::raw(base.display().to_string()),
    ]);
}

/// Append a consistently styled shortcut token for header help text.
pub(crate) fn push_shortcut_span<'a>(spans: &mut impl Extend<Span<'a>>, code: &str) {
    let dim = Style::new().dim();
    let key = Style::new().fg(Color::Yellow);

    spans.extend([
        Span::styled("[", dim),
        Span::styled(code.to_owned(), key),
        Span::styled("]", dim),
    ])
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ratatui::text::Line;

    use super::*;

    #[test]
    fn highlight_match_indices_overlays_existing_span_styles() {
        let spans = highlight(
            vec![
                Span::styled("ab", Style::new().dim()),
                Span::styled("cd", Style::new().fg(Color::Green)),
            ],
            &[1, 2],
        );

        assert_eq!(
            spans,
            vec![
                Span::styled("a", Style::new().dim()),
                Span::styled("bc", HIGHLIGHT),
                Span::styled("d", Style::new().fg(Color::Green)),
            ]
        );
    }

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
