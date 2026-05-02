// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Widget;

pub(super) fn widget(query: &str) -> impl Widget {
    Line::from(vec![
        Span::styled("session: ", Style::new().dim()),
        Span::raw(query.to_owned()),
    ])
}
