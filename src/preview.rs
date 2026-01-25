use std::ops::Deref;

/// Generates a preview of the content in `panes` with a maximum height of `preview_height`.
///
/// `preview_width` is used for rendering separators and omitted-pane notes. Pane content lines are
/// passed through as-is.
///
/// The `preview_height` is divided equally among the panes. If doing so results in a height of
/// less than `min_pane_height` for any pane, later panes will be omitted from the preview until
/// there is enough space to share among the remaining panes.
///
/// If panes have been omitted, a note will be added at the end of the preview indicating how many
/// panes were not shown.
pub(crate) fn preview<I, P>(
    preview_width: usize,
    preview_height: usize,
    min_pane_height: usize,
    mut panes: I,
) -> String
where
    I: ExactSizeIterator<Item = P>,
    P: Deref<Target = Vec<String>>,
{
    // Too small, or no data to show.
    if preview_height < 2 || min_pane_height < 2 || panes.len() == 0 {
        return "".to_owned();
    }

    let mut available_height = preview_height;
    let mut max_panes = available_height / min_pane_height;

    // If there are more panes than can be shown, reserve space for the omitted panes note, and
    // recalculate the number of panes that can be shown assuming that space is reserved.
    if max_panes < panes.len() {
        available_height -= 2;
        max_panes = available_height / min_pane_height;
    }

    // Extreme case: There is not enough space to show even one pane as well as the omitted panes
    // note. In this case, just show the omitted panes note.
    if max_panes == 0 {
        let s = if panes.len() == 1 { "" } else { "s" };
        let msg = format!("{} pane{s}", panes.len());
        return format!("{msg:^preview_width$}");
    }

    let mut preview = Vec::with_capacity(preview_height);
    let separator = format!("{:─^preview_width$}", "");
    let mut needs_separator = false;
    let mut to_render = max_panes.min(panes.len());
    let to_omit = panes.len() - to_render;

    while to_render > 0 {
        if needs_separator {
            available_height -= 1;
            preview.push(separator.clone());
        } else {
            needs_separator = true;
        }

        // SAFETY: `to_render <= panes.len()` by definition.
        let pane = panes.next().unwrap();
        let pane_height = available_height / to_render;

        // If the pane contains more content than can fit in the allotted height, favour the last
        // lines, because for typical command-line applications, this is the most recent output.
        preview.extend(pane.iter().rev().take(pane_height).rev().cloned());

        // Pad with empty lines if necessary.
        for _ in 0..pane_height.saturating_sub(pane.len()) {
            preview.push("".to_owned());
        }

        available_height -= pane_height;
        to_render -= 1;
    }

    debug_assert!(needs_separator, "a pane should have been rendered");
    if to_omit > 0 {
        preview.push(separator);
        let s = if to_omit == 1 { "" } else { "s" };
        let msg = format!("+{} pane{s}", to_omit);
        preview.push(format!("{msg:^preview_width$}"));
    }

    preview.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|line| (*line).to_string()).collect()
    }

    #[test]
    fn returns_empty_for_too_small_preview() {
        let panes = vec![pane(&["a"]), pane(&["b"])];
        let rendered = preview(10, 1, 2, panes.iter());
        insta::assert_snapshot!(rendered, @"");
    }

    #[test]
    fn returns_empty_for_too_small_min_height() {
        let panes = vec![pane(&["a"]), pane(&["b"])];
        let rendered = preview(10, 4, 1, panes.iter());
        insta::assert_snapshot!(rendered, @"");
    }

    #[test]
    fn returns_empty_for_no_panes() {
        let panes: Vec<Vec<String>> = Vec::new();
        let rendered = preview(10, 4, 2, panes.iter());
        insta::assert_snapshot!(rendered, @"");
    }

    #[test]
    fn shows_omitted_note_when_no_room_for_pane() {
        let panes = vec![pane(&["a"]), pane(&["b"]), pane(&["c"])];
        let rendered = preview(12, 3, 2, panes.iter());
        insta::assert_snapshot!(rendered, @"  3 panes");
    }

    #[test]
    fn renders_single_pane_without_omission() {
        let panes = vec![pane(&["one", "two"])];
        let rendered = preview(8, 4, 2, panes.iter());
        insta::assert_snapshot!(rendered, @r###"
        one
        two
        "###);
    }

    #[test]
    fn allocates_height_evenly_and_uses_last_lines() {
        let panes = vec![pane(&["a1", "a2", "a3"]), pane(&["b1", "b2", "b3", "b4"])];
        let rendered = preview(6, 6, 2, panes.iter());
        insta::assert_snapshot!(rendered, @r###"
        a1
        a2
        a3
        ──────
        b3
        b4
        "###);
    }

    #[test]
    fn pads_with_empty_lines_when_pane_has_less_content() {
        let panes = vec![pane(&["a"]), pane(&["b", "c", "d"])];
        let rendered = preview(5, 6, 2, panes.iter());
        insta::assert_snapshot!(rendered, @r###"
        a


        ─────
        c
        d
        "###);
    }

    #[test]
    fn omits_later_panes_and_appends_note() {
        let panes = vec![
            pane(&["a1", "a2"]),
            pane(&["b1", "b2"]),
            pane(&["c1", "c2"]),
        ];
        let rendered = preview(8, 5, 2, panes.iter());
        insta::assert_snapshot!(rendered, @r###"
        a1
        a2

        ────────
        +2 panes
        "###);
    }

    #[test]
    fn uses_plural_for_multiple_omitted_panes() {
        let panes = vec![pane(&["a"]), pane(&["b"]), pane(&["c"]), pane(&["d"])];
        let rendered = preview(10, 7, 2, panes.iter());
        insta::assert_snapshot!(rendered, @r###"
        a

        ──────────
        b

        ──────────
         +2 panes
        "###);
    }
}
