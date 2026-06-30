// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Shared span-construction helpers.

use std::path::MAIN_SEPARATOR;
use std::path::Path;

use ratatui::style::Style;
use ratatui::style::Stylize as _;
use ratatui::text::Span;

use crate::app::highlight::Highlight;
use crate::path::TruncatedExt as _;

/// Append a compact repository path, dimming the parent prefix and leaving the basename undimmed.
///
/// `hl` indicates which part of the path should be highlighted as part of the match. All
/// highlighted parts are kept, but otherwise only the separators and initial non `.` characters of
/// the parent path are kept.
pub(crate) fn push_repo_path_spans<'a, F>(
    spans: &mut impl Extend<Span<'a>>,
    repo: &Path,
    hl: &mut Highlight<F>,
) where
    F: Fn(Style) -> Style,
{
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
    use std::path::PathBuf;

    use ratatui::style::Stylize as _;
    use ratatui::text::Line;

    use crate::app::highlight::Highlight;

    use super::*;

    const HIGHLIGHT: Style = Style::new().blue().bold();
    const SEP: &str = MAIN_SEPARATOR_STR;

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
        let mut highlight = Highlight::new(vec![3, 9, 18], |_| HIGHLIGHT);
        push_repo_path_spans(&mut line, &path, &mut highlight);

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
