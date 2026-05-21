// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

use ratatui::style::Stylize as _;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Widget;

pub(super) fn widget(query: &str) -> impl Widget {
    Line::from(vec![
        Span::raw("session: ").dim(),
        Span::raw(query.to_owned()),
    ])
}
