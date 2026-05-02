// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Widget for filling a rectangular area with a repeated character.

use ratatui::prelude::Buffer;
use ratatui::prelude::Rect;
use ratatui::widgets::Widget;

/// A widget that fills every cell in its area with a repeated character.
pub(crate) struct Block(char);

impl Block {
    /// Create a block that fills its area with `character`.
    pub(crate) const fn new(character: char) -> Self {
        Self(character)
    }
}

impl Widget for Block {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.is_empty() {
            return;
        }

        let mut symbol = [0; 4];
        let symbol = self.0.encode_utf8(&mut symbol);

        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_symbol(symbol);
                }
            }
        }
    }
}
