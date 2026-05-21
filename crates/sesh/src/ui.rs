// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Shared UI rendering helpers.

use std::iter::Peekable;
use std::path::MAIN_SEPARATOR;
use std::path::Path;
use std::vec;

use ratatui::style::Style;
use ratatui::style::Stylize as _;
use ratatui::text::Span;

use crate::path::TruncatedExt as _;

static HIGHLIGHT: Style = Style::new().blue().bold();

/// Stateful highlighter for character indices matched by the active fuzzy query.
pub(crate) struct Highlight {
    indices: Peekable<vec::IntoIter<u32>>,
    processed: u32,
}

impl Highlight {
    /// Construct a highlighter from character positions in matcher text.
    pub(crate) fn new(mut indices: Vec<u32>) -> Self {
        indices.sort_unstable();
        indices.dedup();

        Self {
            indices: indices.into_iter().peekable(),
            processed: 0,
        }
    }

    /// An instance that applies no highlighting (spans are returned unchanged).
    pub(crate) fn none() -> Self {
        Self::new(Vec::new())
    }

    /// Overlay fuzzy match highlighting onto `span`, preserving its style when unmatched.
    pub(crate) fn highlight(&mut self, span: Span<'static>) -> Vec<Span<'static>> {
        let mut output = Vec::new();
        let mut chars = span.content.chars().peekable();

        while chars.peek().is_some() {
            let next = self.indices.peek().copied().unwrap_or(u32::MAX);
            debug_assert!(next >= self.processed, "out-of-order highlight index");

            // Consume all non-highlighted characters before the next matched character.
            let mut plain = String::new();
            while let Some(&ch) = chars.peek()
                && self.processed < next
            {
                plain.push(ch);
                chars.next();
                self.processed += 1;
            }

            if !plain.is_empty() {
                output.push(Span::styled(plain, span.style));
            }

            // Gather the prefix of highlighted characters, overriding their style.
            let mut highlighted = String::new();
            while let (Some(&ch), Some(&ix)) = (chars.peek(), self.indices.peek())
                && ix == self.processed
            {
                highlighted.push(ch);
                chars.next();
                self.indices.next();
                self.processed += 1;
            }

            if !highlighted.is_empty() {
                output.push(Span::styled(highlighted, HIGHLIGHT));
            }
        }

        output
    }
}

/// Append a compact repository path, dimming the parent prefix and leaving the basename undimmed.
///
/// `hl` indicates which part of the path should be highlighted as part of the match. All
/// highlighted parts are kept, but otherwise only the separators and initial non `.` characters of
/// the parent path are kept.
pub(crate) fn push_repo_path_spans<'a>(
    spans: &mut impl Extend<Span<'a>>,
    repo: &Path,
    hl: &mut Highlight,
) {
    let repo = repo.truncated();
    let (parent, base) = repo.split_last();

    let mut parent = parent.display().to_string();
    if !parent.is_empty() && !parent.ends_with(MAIN_SEPARATOR) {
        parent.push(MAIN_SEPARATOR);
    }

    let dim = Style::new().dim();
    let parent = hl.highlight(Span::raw(parent).dim());

    let mut cs = parent
        .iter()
        .flat_map(|s| s.content.chars().map(|ch| (ch, &s.style)))
        .peekable();

    let mut separator_distance = 0;
    while let Some(&(_, base)) = cs.peek() {
        let mut content = String::new();
        while let Some(&(ch, style)) = cs.peek()
            && style == base
        {
            if ch == MAIN_SEPARATOR {
                separator_distance = 0;
            } else if ch == '.' && separator_distance == 0 {
                // treat a leading `.` as part of the separator.
                separator_distance = 0;
            } else {
                separator_distance += 1;
            }

            if style != &dim || separator_distance <= 1 {
                content.push(ch);
            }

            cs.next();
        }

        spans.extend([Span::styled(content, *base)]);
    }

    spans.extend(hl.highlight(Span::raw(base.display().to_string())));
}

/// Append a consistently styled shortcut token for header help text.
pub(crate) fn push_shortcut_span<'a>(spans: &mut impl Extend<Span<'a>>, code: &str) {
    spans.extend([
        Span::raw("[").dim(),
        Span::raw(code.to_owned()).yellow(),
        Span::raw("]").dim(),
    ])
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::MAIN_SEPARATOR_STR;
    use std::path::Path;
    use std::path::PathBuf;

    use ratatui::style::Stylize as _;
    use ratatui::text::Line;

    use super::*;

    const SEP: &str = MAIN_SEPARATOR_STR;

    #[test]
    fn highlight_match_indices_overlays_existing_span_styles() {
        let mut hl = Highlight::new(vec![2, 1, 1]);
        let mut spans = hl.highlight(Span::raw("ab").dim());
        spans.extend(hl.highlight(Span::raw("cd").green()));

        assert_eq!(
            spans,
            vec![
                Span::raw("a").dim(),
                Span::styled("b", HIGHLIGHT),
                Span::styled("c", HIGHLIGHT),
                Span::raw("d").green(),
            ]
        );
    }

    #[test]
    fn repo_path_compacts_absolute_intermediate_components() {
        let mut line = Line::default();
        let path = PathBuf::from(SEP).join("tmp").join("foo").join("bar");

        push_repo_path_spans(&mut line, &path, &mut Highlight::none());

        assert_eq!(
            line.spans,
            vec![
                Span::raw(format!("{SEP}t{SEP}f{SEP}")).dim(),
                Span::raw("bar"),
            ]
        );
    }

    #[test]
    fn repo_path_compacts_hidden_intermediate_components_without_dropping_them() {
        let mut line = Line::default();
        let path = PathBuf::from("~").join(".config").join("nvim");

        push_repo_path_spans(&mut line, &path, &mut Highlight::none());

        assert_eq!(
            line.spans,
            vec![Span::raw(format!("~{SEP}.c{SEP}")).dim(), Span::raw("nvim"),]
        )
    }

    #[test]
    fn repo_path_compacts_truncated_home_relative_paths() {
        let Some(home) = env::home_dir().map(|home| home.canonicalize().unwrap_or(home)) else {
            return;
        };

        let mut line = Line::default();
        let path = home.join("Code").join("foo").join("bar");

        push_repo_path_spans(&mut line, &path, &mut Highlight::none());

        assert_eq!(
            line.spans,
            vec![
                Span::raw(format!("~{SEP}C{SEP}f{SEP}")).dim(),
                Span::raw("bar"),
            ]
        );
    }

    #[test]
    fn repo_path_expands_matched_parent_characters() {
        let mut line = Line::default();

        let path = PathBuf::from(SEP)
            .join("Users")
            .join("dev")
            .join("Code")
            .join("sesh");

        // 00000000001111111111
        // 01234567890123456789
        //    |     |        |
        // /Users/dev/Code/sesh
        let mut hl = Highlight::new(vec![3, 9, 18]);
        push_repo_path_spans(&mut line, &path, &mut hl);

        assert_eq!(
            line.spans,
            vec![
                Span::raw(format!("{SEP}U")).dim(),
                Span::styled("e", HIGHLIGHT),
                Span::raw(format!("{SEP}d")).dim(),
                Span::styled("v", HIGHLIGHT),
                Span::raw(format!("{SEP}C{SEP}")).dim(),
                Span::raw("se"),
                Span::styled("s", HIGHLIGHT),
                Span::raw("h"),
            ]
        );
    }

    #[test]
    fn repo_path_keeps_relative_basename_unprefixed() {
        let mut line = Line::default();

        push_repo_path_spans(&mut line, Path::new("alpha"), &mut Highlight::none());

        assert_eq!(line.spans[0].content, "alpha");
    }

    #[test]
    fn repo_path_parent_dir() {
        let mut line = Line::default();

        let path = PathBuf::from("..").join("repo");

        push_repo_path_spans(&mut line, &path, &mut Highlight::none());

        assert_eq!(
            line.spans,
            vec![Span::raw(format!("..{SEP}")).dim(), Span::raw("repo"),]
        );
    }

    #[test]
    fn repo_path_root_parent() {
        let mut line = Line::default();

        let path = PathBuf::from(MAIN_SEPARATOR_STR).join("repo");

        push_repo_path_spans(&mut line, &path, &mut Highlight::none());

        assert_eq!(
            line.spans,
            vec![Span::raw(SEP.to_owned()).dim(), Span::raw("repo"),]
        );
    }
}
