// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! SVG rendering for styled tmux pane snapshots.

use std::fmt::Write as _;
use std::ops::Range;

use regex::Regex;
use unicode_width::UnicodeWidthStr as _;
use vt100::Color;

use crate::parser;

const BASELINE: u16 = 14;
const CELL_HEIGHT: u16 = 18;
const CELL_WIDTH_PX: f64 = 8.4;
const MARGIN: u16 = 10;

/// Captured terminal frame after filters have been applied to both text and cells.
pub(crate) struct Frame {
    cells: Vec<Cell>,
    cols: u16,
    rows: u16,
    text: String,
}

/// Color palette to use when rendering terminal attributes.
#[derive(Clone, Copy)]
pub(crate) enum Theme {
    Dark,
    Light,
}

/// Mutable copy of `vt100::Cell`.
#[derive(Clone, Default)]
struct Cell {
    bg: Color,
    bold: bool,
    dim: bool,
    fg: Color,
    inverse: bool,
    italic: bool,
    text: String,
    underline: bool,
    wide_continuation: bool,
}

/// Concrete colors used to translate terminal attributes into SVG colors.
struct Palette {
    colors: [&'static str; 16],
    default_bg: &'static str,
    default_fg: &'static str,
}

/// SVG text attributes shared by a contiguous run of terminal cells.
#[derive(Clone, PartialEq, Eq)]
struct TextStyle {
    bold: bool,
    dim: bool,
    fill: String,
    italic: bool,
    underline: bool,
}

impl Frame {
    /// Parse a tmux `capture-pane -p -e` payload into a styled cell grid, then apply filters.
    pub(crate) fn parse(ansi: &[u8], rows: u16, cols: u16, filters: &[parser::Filter]) -> Self {
        let mut parser = vt100::Parser::new(rows, cols, 0);
        parser.process(ansi);
        let screen = parser.screen();

        let mut cells = extract_cells(screen, rows, cols);
        apply_filters(&mut cells, rows, cols, filters);
        let text = plaintext(&cells, rows, cols);

        Self {
            cells,
            cols,
            rows,
            text,
        }
    }

    /// Render this frame as a deterministic SVG image using the requested theme.
    pub(crate) fn render_svg(&self, theme: Theme) -> String {
        let theme = Palette::new(theme);
        let width = (f64::from(self.cols) * CELL_WIDTH_PX).ceil() as u32 + u32::from(MARGIN) * 2;
        let height = u32::from(self.rows) * u32::from(CELL_HEIGHT) + u32::from(MARGIN) * 2;
        let mut out = String::new();

        out.push_str("<svg");
        write!(out, r#" width="{width}px" height="{height}px""#).unwrap();
        out.push_str(r#" xmlns="http://www.w3.org/2000/svg">"#);
        writeln!(out).unwrap();

        out.push_str(r#"<rect width="100%" height="100%""#);
        writeln!(out, r#" fill="{}"/>"#, theme.default_bg).unwrap();

        out.push_str("<style>text{font-family:'Iosevka Term SS15','Iosevka Term',");
        out.push_str("ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;");
        out.push_str("font-size:14px;white-space:pre}</style>\n");

        self.render_bg(&mut out, &theme);
        self.render_fg(&mut out, &theme);

        out.push_str("</svg>\n");
        out
    }

    /// Return the plaintext terminal contents for the markdown transcript and settle checks.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// Append SVG text rows that paint non-default cell backgrounds.
    fn render_bg(&self, out: &mut String, theme: &Palette) {
        for row in 0..self.rows {
            let row_cells = row_cells(&self.cells, self.cols, row);
            let runs = rle_bg(row_cells, theme);
            if matches!(runs.as_slice(), [(bg, _)] if bg == theme.default_bg) {
                continue;
            }

            let x = MARGIN;
            let y = MARGIN + row * CELL_HEIGHT + BASELINE;

            write!(
                out,
                r#"<text x="{x}" y="{y}" xml:space="preserve" aria-hidden="true">"#
            )
            .unwrap();

            for (bg, len) in runs {
                let text = "█".repeat(len);
                write!(out, "<tspan fill=\"{bg}\" stroke=\"{bg}\">{text}</tspan>").unwrap();
            }

            out.push_str("</text>\n");
        }
    }

    /// Append one SVG text element per row, with styled tspans inside it.
    fn render_fg(&self, out: &mut String, theme: &Palette) {
        for row in 0..self.rows {
            let row_cells = row_cells(&self.cells, self.cols, row);
            let runs = rle_fg(row_cells, theme);
            if matches!(runs.as_slice(), [(_, t)] if t.trim().is_empty()) {
                continue;
            }

            let x = MARGIN;
            let y = MARGIN + row * CELL_HEIGHT + BASELINE;
            write!(out, r#"<text x="{x}" y="{y}" xml:space="preserve">"#).unwrap();

            for (style, text) in runs {
                write!(
                    out,
                    r#"<tspan fill="{fill}"{weight}{style}{opacity}{underline}>{text}</tspan>"#,
                    fill = style.fill,
                    weight = style.font_weight_attr(),
                    style = style.font_style_attr(),
                    opacity = style.opacity_attr(),
                    underline = style.underline_attr(),
                    text = escape_xml(&text)
                )
                .unwrap();
            }

            out.push_str("</text>\n");
        }
    }
}

impl Palette {
    /// Construct the concrete color palette for `theme`.
    fn new(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self {
                colors: [
                    "#000000", "#ff5360", "#59d499", "#ffc531", "#56c2ff", "#cf2f98", "#52eee5",
                    "#ffffff", "#4c4c4c", "#ff6363", "#59d499", "#ffc531", "#56c2ff", "#cf2f98",
                    "#52eee5", "#ffffff",
                ],
                default_bg: "#1a1a1a",
                default_fg: "#ffffff",
            },
            Theme::Light => Self {
                colors: [
                    "#000000", "#b12424", "#006b4f", "#f8a300", "#138af2", "#9a1b6e", "#3eb8bf",
                    "#bfbfbf", "#000000", "#b12424", "#006b4f", "#f8a300", "#138af2", "#9a1b6e",
                    "#3eb8bf", "#ffffff",
                ],
                default_bg: "#ffffff",
                default_fg: "#000000",
            },
        }
    }

    /// Resolve a cell background after applying inverse-video semantics.
    fn bg(&self, cell: &Cell) -> String {
        if cell.inverse {
            self.color(cell.fg, self.default_fg)
        } else {
            self.color(cell.bg, self.default_bg)
        }
    }

    /// Convert a vt100 color into a CSS hex color.
    fn color(&self, color: Color, default: &str) -> String {
        match color {
            Color::Default => default.to_owned(),
            Color::Idx(index) => self.colors[usize::from(index.min(15))].to_owned(),
            Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        }
    }

    /// Resolve a cell foreground after applying inverse-video semantics.
    fn fg(&self, cell: &Cell) -> String {
        if cell.inverse {
            self.color(cell.bg, self.default_bg)
        } else {
            self.color(cell.fg, self.default_fg)
        }
    }
}

impl TextStyle {
    /// Construct an SVG text style from a terminal cell and render palette.
    fn from_cell(cell: &Cell, theme: &Palette) -> Self {
        Self {
            bold: cell.bold,
            dim: cell.dim,
            fill: theme.fg(cell),
            italic: cell.italic,
            underline: cell.underline,
        }
    }

    /// Return the SVG font-style attribute fragment for this run.
    fn font_style_attr(&self) -> &'static str {
        if self.italic {
            " font-style=\"italic\""
        } else {
            ""
        }
    }

    /// Return the SVG font-weight attribute fragment for this run.
    fn font_weight_attr(&self) -> &'static str {
        if self.bold {
            " font-weight=\"700\""
        } else {
            ""
        }
    }

    /// Return the SVG opacity attribute fragment for this run.
    fn opacity_attr(&self) -> &'static str {
        if self.dim { " opacity=\"0.7\"" } else { "" }
    }

    /// Return the SVG text-decoration attribute fragment for this run.
    fn underline_attr(&self) -> &'static str {
        if self.underline {
            " text-decoration=\"underline\""
        } else {
            ""
        }
    }
}

/// Apply all row-local replacement filters to the cell grid.
fn apply_filters(cells: &mut [Cell], rows: u16, cols: u16, filters: &[parser::Filter]) {
    for filter in filters {
        for row in 0..rows {
            let mut text = String::new();
            for cell in row_cells(cells, cols, row) {
                text.push_str(&cell.text);
            }

            let cells = row_cells_mut(cells, cols, row);
            for range in filter_ranges(&text, &filter.patt) {
                let mut start = text[..range.start].width();
                let len = text[range].width();
                if len == 0 || start >= cells.len() {
                    continue;
                }

                let mut end = (start + len).min(cells.len());

                while start < cells.len() && cells[start].wide_continuation && start > 0 {
                    start -= 1;
                }

                while end < cells.len() && cells[end].wide_continuation {
                    end += 1;
                }

                for cell in &mut cells[start..end] {
                    cell.text = filter.paint.clone();
                    cell.wide_continuation = false;
                }
            }
        }
    }
}

/// Return an empty terminal cell with printable text padding.
fn blank_cell() -> Cell {
    Cell {
        text: " ".to_owned(),
        ..Cell::default()
    }
}

/// Escape text so it can be embedded in an SVG text node.
fn escape_xml(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }

    out
}

/// Copy the visible `screen` into a mutable cell grid.
fn extract_cells(screen: &vt100::Screen, rows: u16, cols: u16) -> Vec<Cell> {
    let mut cells = Vec::with_capacity(usize::from(rows) * usize::from(cols));
    for row in 0..rows {
        for col in 0..cols {
            let Some(cell) = screen.cell(row, col) else {
                cells.push(blank_cell());
                continue;
            };

            let content = if cell.is_wide_continuation() {
                ""
            } else if cell.contents().is_empty() {
                " "
            } else {
                cell.contents()
            };

            cells.push(Cell {
                bg: cell.bgcolor(),
                bold: cell.bold(),
                dim: cell.dim(),
                fg: cell.fgcolor(),
                inverse: cell.inverse(),
                italic: cell.italic(),
                text: content.to_owned(),
                underline: cell.underline(),
                wide_continuation: cell.is_wide_continuation(),
            });
        }
    }

    cells
}

/// Return sorted, merged byte ranges matched by a filter regex or its capture groups.
fn filter_ranges(input: &str, regex: &Regex) -> Vec<Range<usize>> {
    let mut ranges = vec![];
    for captures in regex.captures_iter(input) {
        if captures.len() == 1 {
            let m = captures.get_match();
            ranges.push(m.start()..m.end());
        }

        for m in captures.iter().skip(1).flatten() {
            ranges.push(m.start()..m.end());
        }
    }

    ranges.sort_by_key(|r| (r.start, r.end));
    let mut merged: Vec<Range<usize>> = vec![];
    for range in ranges {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }

    merged
}

/// Build the plaintext transcript from cells after replacement filters have run.
fn plaintext(cells: &[Cell], rows: u16, cols: u16) -> String {
    let mut text = String::new();
    for row in 0..rows {
        let mut line = String::new();
        for cell in row_cells(cells, cols, row) {
            line.push_str(&cell.text);
        }

        text.push_str(line.trim_end());
        text.push('\n');
    }

    text
}

/// Return run-length encoded backgrounds for a row.
fn rle_bg(cells: &[Cell], theme: &Palette) -> Vec<(String, usize)> {
    let mut runs = vec![];
    for cell in cells {
        let bg = theme.bg(cell);
        match runs.last_mut() {
            Some((last_bg, len)) if last_bg == &bg => *len += 1,
            _ => runs.push((bg, 1)),
        }
    }

    runs
}

/// Return style runs for foreground text in a row, accumulating content per run.
fn rle_fg(cells: &[Cell], theme: &Palette) -> Vec<(TextStyle, String)> {
    let mut runs: Vec<(TextStyle, String)> = vec![];
    for cell in cells {
        if cell.wide_continuation {
            continue;
        }

        let style = TextStyle::from_cell(cell, theme);
        match runs.last_mut() {
            Some((last_style, text)) if last_style == &style => text.push_str(&cell.text),
            _ => runs.push((style, cell.text.clone())),
        }
    }

    runs
}

/// Return the cells for `row`.
fn row_cells(cells: &[Cell], cols: u16, row: u16) -> &[Cell] {
    let start = usize::from(row) * usize::from(cols);
    &cells[start..start + usize::from(cols)]
}

/// Return the mutable cells for `row`.
fn row_cells_mut(cells: &mut [Cell], cols: u16, row: u16) -> &mut [Cell] {
    let start = usize::from(row) * usize::from(cols);
    &mut cells[start..start + usize::from(cols)]
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::*;

    #[test]
    fn filter_replaces_both_cells_of_a_wide_character() {
        let mut cells = vec![
            test_cell("界"),
            Cell {
                text: " ".to_owned(),
                wide_continuation: true,
                ..Cell::default()
            },
            test_cell("x"),
        ];
        let filter = parser::Filter {
            patt: Regex::new("界").unwrap(),
            paint: "*".to_owned(),
        };

        apply_filters(&mut cells, 1, 3, &[filter]);

        assert_eq!(cells[0].text, "*");
        assert_eq!(cells[1].text, "*");
        assert_eq!(cells[2].text, "x");
        assert!(!cells[0].wide_continuation);
        assert!(!cells[1].wide_continuation);
    }

    #[test]
    fn plaintext_trims_rows_containing_only_spaces() {
        let cells = vec![test_cell(" "), test_cell(" ")];

        assert_eq!(plaintext(&cells, 1, 2), "\n");
    }

    fn test_cell(text: &str) -> Cell {
        Cell {
            text: text.to_owned(),
            ..Cell::default()
        }
    }
}
