// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Rendering for the active prompt line.

use ratatui::style::Stylize as _;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Widget;

/// Build the prompt widget for `label` and `query`.
pub(super) fn widget(label: &str, query: &str) -> impl Widget {
    Line::from(vec![
        Span::raw(format!("{label}: ")).dim(),
        Span::raw(query.to_owned()),
    ])
}
