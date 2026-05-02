// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Calculates geometry for app widgets within a given display area.

use ratatui::layout::Rect;

/// Percentage of space to give to the preview pane when in horizontal split mode.
const PERC_H_PREVIEW: u16 = 60;

/// Percentage of space to give to the preview pane when in vertical split mode.
const PERC_V_PREVIEW: u16 = 40;

/// The largest the preview pane gets.
const WIDTH_MAX_PREVIEW: u16 = 100;

/// The minimum width to support a vertical split layout (session list and preview side-by-side).
const WIDTH_MIN_VSPLIT: u16 = 160;

/// Regions to render each component into, each houses its own widget. When there is enough
/// horizontal space, the layout is as follows:
///
/// ```text
/// +-----------------+-+--------------+
/// | prompt          |s| preview      |
/// +-+---------------+c| ...          |
/// |l| header        |r|              |
/// +-+---------------+o|              |
/// | sessions        |l|              |
/// | ...             |l|              |
/// |                 | |              |
/// |                 | |              |
/// |                 | |              |
/// +-----------------+-+--------------+
/// ```
///
/// When the display is too narrow, it stacks vertically:
///
/// ```text
/// +--------------------------+
/// | prompt                   |
/// +-+------------------------+
/// |l| header                 |
/// +-+----------------------+-|
/// | sessions               |s|
/// | ...                    |c|
/// |                        |r|
/// +------------------------+-+
/// | separator                |
/// +--------------------------+
/// | preview                  |
/// | ...                      |
/// |                          |
/// +--------------------------+
/// ```
pub(crate) struct Layout {
    pub(crate) header: Rect,
    pub(crate) loading: Rect,
    pub(crate) preview: Option<Rect>,
    pub(crate) prompt: Rect,
    pub(crate) scroll: Rect,
    pub(crate) separator: Option<Rect>,
    pub(crate) sessions: Rect,
}

impl Layout {
    pub(crate) fn new(area: Rect, preview: bool) -> Self {
        use ratatui::layout::Constraint as C;
        use ratatui::layout::Direction as D;
        use ratatui::layout::Layout as L;

        if !preview {
            let cols = L::default()
                .direction(D::Horizontal)
                .constraints([C::Min(0), C::Length(1)])
                .split(area);

            let &[content, scroll] = &cols[..] else {
                panic!("expected two columns in the layout");
            };

            let rows = L::default()
                .direction(D::Vertical)
                .constraints([C::Length(1), C::Length(1), C::Min(0)])
                .split(content);

            let &[prompt, header, sessions] = &rows[..] else {
                panic!("expected three rows in the layout")
            };

            let cols = L::default()
                .direction(D::Horizontal)
                .constraints([C::Length(1), C::Min(0)])
                .split(header);

            let &[loading, header] = &cols[..] else {
                panic!("expected two columns in header");
            };

            Self {
                header,
                loading,
                preview: None,
                prompt,
                scroll,
                separator: None,
                sessions,
            }
        } else if area.width > 100 * WIDTH_MAX_PREVIEW / PERC_V_PREVIEW {
            let cols = L::default()
                .direction(D::Horizontal)
                .constraints([C::Min(0), C::Length(1), C::Length(WIDTH_MAX_PREVIEW)])
                .split(area);

            let &[content, scroll, preview] = &cols[..] else {
                panic!("expected three columns in the layout");
            };

            let rows = L::default()
                .direction(D::Vertical)
                .constraints([C::Length(1), C::Length(1), C::Min(0)])
                .split(content);

            let &[prompt, header, sessions] = &rows[..] else {
                panic!("expected three rows in the layout")
            };

            let cols = L::default()
                .direction(D::Horizontal)
                .constraints([C::Length(1), C::Min(0)])
                .split(header);

            let &[loading, header] = &cols[..] else {
                panic!("expected two columns in header");
            };

            Self {
                header,
                loading,
                preview: Some(preview),
                prompt,
                scroll,
                separator: None,
                sessions,
            }
        } else if area.width >= WIDTH_MIN_VSPLIT {
            let cols = L::default()
                .direction(D::Horizontal)
                .constraints([
                    C::Percentage(100 - PERC_V_PREVIEW),
                    C::Length(1),
                    C::Percentage(PERC_V_PREVIEW),
                ])
                .split(area);

            let &[content, scroll, preview] = &cols[..] else {
                panic!("expected three columns in the layout");
            };

            let rows = L::default()
                .direction(D::Vertical)
                .constraints([C::Length(1), C::Length(1), C::Min(0)])
                .split(content);

            let &[prompt, header, sessions] = &rows[..] else {
                panic!("expected three rows in the layout")
            };

            let cols = L::default()
                .direction(D::Horizontal)
                .constraints([C::Length(1), C::Min(0)])
                .split(header);

            let &[loading, header] = &cols[..] else {
                panic!("expected two columns in header");
            };

            Self {
                header,
                loading,
                preview: Some(preview),
                prompt,
                scroll,
                separator: None,
                sessions,
            }
        } else {
            let rows = L::default()
                .direction(D::Vertical)
                .constraints([C::Min(0), C::Length(1), C::Percentage(PERC_H_PREVIEW)])
                .split(area);

            let &[content, separator, preview] = &rows[..] else {
                panic!("expected three rows in the layout");
            };

            let rows = L::default()
                .direction(D::Vertical)
                .constraints([C::Length(1), C::Length(1), C::Min(0)])
                .split(content);

            let &[prompt, header, sessions] = &rows[..] else {
                panic!("expected three rows in the layout")
            };

            let cols = L::default()
                .direction(D::Horizontal)
                .constraints([C::Length(1), C::Min(0)])
                .split(header);

            let &[loading, header] = &cols[..] else {
                panic!("expected two columns in header");
            };

            let cols = L::default()
                .direction(D::Horizontal)
                .constraints([C::Min(0), C::Length(1)])
                .split(sessions);

            let &[sessions, scroll] = &cols[..] else {
                panic!("expected two columns in the layout");
            };

            Self {
                header,
                loading,
                preview: Some(preview),
                prompt,
                scroll,
                separator: Some(separator),
                sessions,
            }
        }
    }
}
