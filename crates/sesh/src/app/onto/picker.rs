// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Rendering for the `onto` revision picker.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget as _;
use regex::Regex;

/// Matches commit header lines in forced-curved `builtin_log_compact` output.
static COMMIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:│ )*[@○◆×](?: │)* {2,}(?P<rev>[a-z]+)(?:\s|$)")
        .expect("valid jj log header regex")
});

/// Picker view over renderable log text for the current repo context.
pub(super) struct Picker {
    text: Text<'static>,
    /// Commit metadata keyed by the commit's starting rendered row.
    #[allow(dead_code)]
    index: BTreeMap<usize, Commit>,
}

/// Mutable state owned by the onto-picker preview surface.
pub(super) type State = ScrollbarState;

/// Search and selection metadata for a commit in the rendered log.
#[derive(Debug, Eq, PartialEq)]
struct Commit {
    /// Flattened rendered lines matched by the picker.
    text: Vec<String>,
    /// Change-id token suitable for passing back to `jj`.
    rev: String,
}

impl Picker {
    /// Create a picker view over renderable `jj log` text.
    pub(super) fn new(text: Text<'static>) -> Self {
        let mut current: Option<(usize, Commit)> = None;
        let mut index = BTreeMap::new();

        for (i, line) in text.lines.iter().enumerate() {
            let line: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

            if let Some(captures) = COMMIT.captures(&line)
                && let Some(rev) = captures.name("rev")
            {
                index.extend(current.take());
                let commit = Commit {
                    rev: rev.as_str().to_owned(),
                    text: vec![line],
                };

                current = Some((i, commit));
            } else if line.trim_start().starts_with('~') {
                index.extend(current.take());
            } else if let Some((_, commit)) = &mut current {
                commit.text.push(line);
            }
        }

        index.extend(current.take());
        Self { text, index }
    }
}

impl StatefulWidget for &Picker {
    type State = State;

    fn render(self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        let area = area.intersection(buf.area);
        if area.is_empty() {
            return;
        }

        buf.set_style(area, self.text.style);
        for (line, line_area) in self.text.lines.iter().zip(area.rows()) {
            line.render(line_area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::text::Line;
    use ratatui::text::Span;

    use super::*;

    macro_rules! assert_index_invariants {
        ($picker:expr $(,)?) => {{
            let picker = $picker;
            for (row, commit) in &picker.index {
                let end = row + commit.text.len();
                assert!(*row < picker.text.lines.len());
                assert!(*row < end);
                assert!(end <= picker.text.lines.len());
                assert!(!commit.text.is_empty());
                assert!(!commit.rev.is_empty());

                let rendered: Vec<_> = picker.text.lines[*row..end]
                    .iter()
                    .map(|line| {
                        let rendered: String = line
                            .spans
                            .iter()
                            .map(|span| span.content.as_ref())
                            .collect();
                        rendered
                    })
                    .collect();
                assert_eq!(commit.text, rendered);
            }
        }};
    }

    macro_rules! assert_summary {
        ($picker:expr, $expected:expr $(,)?) => {{
            assert_index_invariants!($picker);
            assert_eq!(summary($picker), $expected);
        }};
    }

    fn picker(lines: &[&str]) -> Picker {
        let lines: Vec<_> = lines
            .iter()
            .map(|line| Line::raw((*line).to_owned()))
            .collect();
        Picker::new(Text::from(lines))
    }

    fn summary(picker: &Picker) -> Vec<(usize, &str, Vec<&str>)> {
        picker
            .index
            .iter()
            .map(|(start, commit)| {
                (
                    *start,
                    commit.rev.as_str(),
                    commit.text.iter().map(String::as_str).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn flattens_styled_lines_before_indexing() {
        let text = Text::from(vec![
            Line::from(vec![
                Span::raw("@"),
                Span::raw("  abcdefgh"),
                Span::raw(" user@example.com 2026-06-29 aaaaaaaa"),
            ]),
            Line::from(vec![Span::raw("│  styled description")]),
        ]);
        let picker = Picker::new(text);

        assert_summary!(
            &picker,
            vec![(
                0,
                "abcdefgh",
                vec![
                    "@  abcdefgh user@example.com 2026-06-29 aaaaaaaa",
                    "│  styled description",
                ],
            ),]
        );
    }

    #[test]
    fn ignores_unparseable_lines_before_the_first_commit() {
        let picker = picker(&[
            "unexpected banner",
            "~",
            "@  abcdefgh user@example.com 2026-06-29 aaaaaaaa",
        ]);

        assert_summary!(
            &picker,
            vec![(
                2,
                "abcdefgh",
                vec!["@  abcdefgh user@example.com 2026-06-29 aaaaaaaa"],
            ),]
        );
    }

    #[test]
    fn includes_merge_connector_body_lines_in_preceding_commit() {
        let picker = picker(&[
            "@    mergeone user@example.com 2026-06-29 aaaaaaaa",
            "├─╮  merge description",
            "│ ○  rightone user@example.com 2026-06-28 bbbbbbbb",
            "│ │  right description",
            "○ │  leftone user@example.com 2026-06-27 cccccccc",
            "├─╯  left description",
            "◆  zzzzzzzz root() 00000000",
        ]);

        assert_summary!(
            &picker,
            vec![
                (
                    0,
                    "mergeone",
                    vec![
                        "@    mergeone user@example.com 2026-06-29 aaaaaaaa",
                        "├─╮  merge description",
                    ],
                ),
                (
                    2,
                    "rightone",
                    vec![
                        "│ ○  rightone user@example.com 2026-06-28 bbbbbbbb",
                        "│ │  right description",
                    ],
                ),
                (
                    4,
                    "leftone",
                    vec![
                        "○ │  leftone user@example.com 2026-06-27 cccccccc",
                        "├─╯  left description",
                    ],
                ),
                (6, "zzzzzzzz", vec!["◆  zzzzzzzz root() 00000000"]),
            ]
        );
    }

    #[test]
    fn indexes_linear_log_commits_by_starting_row() {
        let picker = picker(&[
            "@  abcdefgh user@example.com 2026-06-29 aaaaaaaa",
            "│  first description",
            "○  ijklmnop user@example.com 2026-06-28 bbbbbbbb",
            "│  second description",
            "◆  zzzzzzzz root() 00000000",
        ]);

        assert_summary!(
            &picker,
            vec![
                (
                    0,
                    "abcdefgh",
                    vec![
                        "@  abcdefgh user@example.com 2026-06-29 aaaaaaaa",
                        "│  first description",
                    ],
                ),
                (
                    2,
                    "ijklmnop",
                    vec![
                        "○  ijklmnop user@example.com 2026-06-28 bbbbbbbb",
                        "│  second description",
                    ],
                ),
                (4, "zzzzzzzz", vec!["◆  zzzzzzzz root() 00000000"]),
            ]
        );
    }

    #[test]
    fn requires_two_spaces_between_graph_and_revision_hint() {
        let picker = picker(&[
            "@ abcdefgh user@example.com 2026-06-29 aaaaaaaa",
            "│ ○ ijklmnop user@example.com 2026-06-28 bbbbbbbb",
            "@  qrstuvwx user@example.com 2026-06-27 cccccccc",
        ]);

        assert_summary!(
            &picker,
            vec![(
                2,
                "qrstuvwx",
                vec!["@  qrstuvwx user@example.com 2026-06-27 cccccccc"],
            ),]
        );
    }

    #[test]
    fn treats_elisions_as_unindexed_gaps() {
        let picker = picker(&[
            "@  abcdefgh user@example.com 2026-06-29 aaaaaaaa",
            "│  first description",
            "~",
            "◆  zzzzzzzz root() 00000000",
        ]);

        assert_summary!(
            &picker,
            vec![
                (
                    0,
                    "abcdefgh",
                    vec![
                        "@  abcdefgh user@example.com 2026-06-29 aaaaaaaa",
                        "│  first description",
                    ],
                ),
                (3, "zzzzzzzz", vec!["◆  zzzzzzzz root() 00000000"]),
            ]
        );
    }
}
