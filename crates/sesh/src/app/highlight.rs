// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Fuzzy-match highlighting helpers.

use std::iter::Peekable;
use std::vec;

use ratatui::style::Style;
use ratatui::text::Span;

pub(super) static HIGHLIGHT: Style = Style::new().blue().bold();

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

#[cfg(test)]
mod tests {
    use ratatui::style::Stylize as _;

    use super::*;

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
}
