// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Rendering for the `onto` revision picker.

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::LazyLock;

use nucleo::Config;
use nucleo::Matcher;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Text;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget as _;
use regex::Regex;

use crate::app::component::scrollbar;
use crate::app::highlight::Highlight;
use crate::model::picker as model;

/// Matches commit header lines in forced-curved `builtin_log_compact` output.
static COMMIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:│ )*(?P<node>[@○◆×])(?: │)* {2,}(?P<rev>[a-z]+)(?:\s|$)")
        .expect("valid jj log header regex")
});

/// Matches elision lines in forced-curved `builtin_log_compact` output.
static ELISION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:│ )*~(?: │)*(?: {2,}|$)").expect("valid jj log elision regex")
});

/// A candidate line from a commit for the fuzzy-finder.
pub(super) struct Candidate {
    /// The zero-based position of this line in the rendered text.
    index: usize,
    /// The flattened rendered text for this line of the commit.
    text: String,
}

/// Picker view over renderable log text for the current repo context.
pub(super) struct Picker {
    text: Text<'static>,
    /// Commit metadata in rendered row order.
    commits: Vec<Commit>,
}

/// Mutable state owned by the onto-picker preview surface.
pub(super) struct State {
    pub(super) model: model::Picker<Candidate>,
    scrollbar: ScrollbarState,
    /// Zero-based index of the selected commit.
    selected: Option<usize>,
}

/// Search and selection metadata for a commit in the rendered log.
#[derive(Debug, Eq, PartialEq)]
struct Commit {
    /// The commit's zero-based starting row in the rendered log.
    start: usize,
    /// Whether this is the current workspace's working-copy commit.
    head: bool,
    /// Flattened rendered lines matched by the picker.
    text: Vec<String>,
    /// Change-id token suitable for passing back to `jj`.
    rev: String,
}

impl Picker {
    /// Create a picker view over renderable `jj log` text.
    pub(super) fn new(text: Text<'static>) -> Self {
        let mut current: Option<Commit> = None;
        let mut commits = Vec::new();

        for (i, line) in text.lines.iter().enumerate() {
            let line: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

            if let Some(captures) = COMMIT.captures(&line)
                && let Some(rev) = captures.name("rev")
            {
                commits.extend(current.take());
                let commit = Commit {
                    start: i,
                    head: captures
                        .name("node")
                        .is_some_and(|node| node.as_str() == "@"),
                    rev: rev.as_str().to_owned(),
                    text: vec![line],
                };

                current = Some(commit);
            } else if ELISION.is_match(&line) {
                commits.extend(current.take());
            } else if let Some(commit) = &mut current {
                commit.text.push(line);
            }
        }

        commits.extend(current.take());

        Self { text, commits }
    }

    /// List all candidate lines for fuzzy-finding.
    pub(super) fn candidates(&self) -> impl Iterator<Item = Candidate> + '_ {
        self.commits.iter().flat_map(|commit| {
            commit
                .text
                .iter()
                .cloned()
                .enumerate()
                .map(move |(line, text)| Candidate {
                    index: commit.start + line,
                    text,
                })
        })
    }
}

impl State {
    /// Initialize commit selection and fuzzy candidates from a loaded picker.
    pub(super) fn initialize(&mut self, picker: &Picker) {
        self.selected = picker
            .commits
            .iter()
            .position(|commit| commit.head)
            .or((!picker.commits.is_empty()).then_some(0));
        self.model.inject(picker.candidates());
    }

    /// Move selection down by one commit.
    pub(super) fn select_next(&mut self) {
        self.selected = self.selected.map(|selected| selected.saturating_add(1));
    }

    /// Move selection up by one commit.
    pub(super) fn select_previous(&mut self) {
        self.selected = self.selected.map(|selected| selected.saturating_sub(1));
    }
}

impl Commit {
    /// Range of lines in the rendered text that belong to this commit.
    fn rows(&self) -> Range<usize> {
        self.start..self.start + self.text.len()
    }
}

impl model::Pickable for Candidate {
    fn text(&self) -> String {
        self.text.clone()
    }
}

impl StatefulWidget for &Picker {
    type State = State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = area.intersection(buf.area);
        if area.is_empty() {
            return;
        }

        let height = area.height as usize;
        let overflow = self.text.lines.len().saturating_sub(height);
        let content = if overflow == 0 { 0 } else { overflow + 1 };

        // Clamp the selected commit.
        state.selected = state
            .selected
            .filter(|_| !self.commits.is_empty())
            .map(|s| s.min(self.commits.len() - 1));

        let selected = state.selected.map(|s| &self.commits[s]);

        // Adjust scroll to keep selected commit visible, and minimize redundant trailing empty
        // rows.
        let mut position = state.scrollbar.get_position().min(overflow);
        if let Some(selected) = selected.map(Commit::rows) {
            position = selected.end.max(position + height) - height;
            position = selected.start.min(position);
        }

        let position = position;
        let viewport = position..(position + height).min(self.text.lines.len());

        // Calculate fuzzy-matching highlights for visible candidate lines.
        let (_, snapshot, _) = state.model.refresh();
        let pattern = snapshot.pattern().column_pattern(0);
        let mut matcher = Matcher::new(Config::DEFAULT);
        let mut highlights = BTreeMap::new();

        for item in snapshot.matched_items(..) {
            if !viewport.contains(&item.data.index) {
                continue;
            }

            let mut indices = Vec::new();
            let text = item.matcher_columns[0].slice(..);

            pattern.indices(text, &mut matcher, &mut indices);
            indices.sort_unstable();
            indices.dedup();

            highlights.insert(item.data.index, indices);
        }

        // Render the lines in the viewport.
        buf.set_style(area, self.text.style);
        for (i, (area, line)) in area.rows().zip(&self.text.lines[viewport]).enumerate() {
            let offset = position + i;
            let indices = highlights.remove(&offset).unwrap_or_default();
            highlight(line, indices).render(area, buf);

            if selected.is_some_and(|commit| commit.rows().contains(&offset)) {
                buf.set_style(area, Style::new().reversed());
            }
        }

        // Render the scrollbar.
        state.scrollbar = state
            .scrollbar
            .content_length(content)
            .viewport_content_length(height)
            .position(position);

        scrollbar::widget().render(area, buf, &mut state.scrollbar);
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            model: model::Picker::new(String::new()),
            scrollbar: ScrollbarState::default(),
            selected: None,
        }
    }
}

/// Return `line` with underlines overlaid at fuzzy-match character positions in `indices`.
fn highlight(line: &Line<'static>, indices: Vec<u32>) -> Line<'static> {
    let mut hl = Highlight::new(indices, Style::underlined);
    let spans = line
        .spans
        .iter()
        .cloned()
        .flat_map(|span| hl.highlight(span))
        .collect();

    Line {
        style: line.style,
        alignment: line.alignment,
        spans,
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
            for commit in &picker.commits {
                assert!(commit.start < picker.text.lines.len());
                assert!(!commit.text.is_empty());
                assert!(!commit.rev.is_empty());

                let rendered: Vec<_> = picker
                    .text
                    .lines
                    .get(commit.rows())
                    .unwrap()
                    .iter()
                    .map(|line| {
                        let rendered: String =
                            line.spans.iter().map(|s| s.content.as_ref()).collect();
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
            .commits
            .iter()
            .map(|commit| {
                (
                    commit.start,
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
    fn initially_selects_head_when_it_is_not_the_first_commit() {
        let picker = picker(&[
            "○  childone user@example.com 2026-06-30 bbbbbbbb",
            "│  child description",
            "@  abcdefgh user@example.com 2026-06-29 aaaaaaaa",
            "│  working-copy description",
            "◆  zzzzzzzz root() 00000000",
        ]);
        let mut state = State::default();

        state.initialize(&picker);

        assert_eq!(state.selected, Some(1));
    }

    #[test]
    fn requires_two_spaces_between_graph_and_rev() {
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

    #[test]
    fn treats_nested_elisions_and_connectors_as_unindexed_gaps() {
        let picker = picker(&[
            "│ ◆  abcdefgh user@example.com 2026-06-29 aaaaaaaa",
            "│ │  first description",
            "│ ~  (elided revisions)",
            "├─╯",
            "│ ○  ijklmnop user@example.com 2026-06-28 bbbbbbbb",
            "├─╯  second description",
        ]);

        assert_summary!(
            &picker,
            vec![
                (
                    0,
                    "abcdefgh",
                    vec![
                        "│ ◆  abcdefgh user@example.com 2026-06-29 aaaaaaaa",
                        "│ │  first description",
                    ],
                ),
                (
                    4,
                    "ijklmnop",
                    vec![
                        "│ ○  ijklmnop user@example.com 2026-06-28 bbbbbbbb",
                        "├─╯  second description",
                    ],
                ),
            ]
        );
    }
}
