// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;

/// Standard styling for a scrollbar in the app.
pub(super) fn widget() -> Scrollbar<'static> {
    Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .track_symbol(Some("│"))
        .thumb_symbol("┃")
}
