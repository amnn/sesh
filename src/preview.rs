use std::ops::Deref;

const ANSI_RESET: &str = "\x1b[0m";

/// Generates a stacked preview for `panes`.
///
/// Each pane contributes up to `pane_summary_height` lines to the output.
///
/// `preview_width` is used for rendering separators. Pane content lines are passed through as-is.
pub(crate) fn preview<I, P>(preview_width: usize, pane_summary_height: usize, panes: I) -> String
where
    I: ExactSizeIterator<Item = P>,
    P: Deref<Target = Vec<String>>,
{
    if pane_summary_height == 0 || panes.len() == 0 {
        return "".to_owned();
    }

    let mut preview = Vec::new();
    let separator = format!("{ANSI_RESET}{:─^preview_width$}", "");

    for pane in panes {
        let summary: Vec<String> = pane
            .deref()
            .iter()
            .rev()
            .filter(|line| !line.trim().is_empty())
            .take(pane_summary_height)
            .cloned()
            .collect();

        if summary.is_empty() {
            continue;
        }

        preview.extend(summary.into_iter().rev());
        preview.push(separator.clone());
    }

    preview.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|line| (*line).to_owned()).collect()
    }

    #[test]
    fn returns_empty_for_no_panes() {
        let panes: Vec<Vec<String>> = Vec::new();
        let rendered = preview(10, 2, panes.iter());
        insta::assert_snapshot!(rendered, @"");
    }

    #[test]
    fn returns_empty_for_zero_summary_height() {
        let panes = vec![pane(&["a"]), pane(&["b"])];
        let rendered = preview(10, 0, panes.iter());
        insta::assert_snapshot!(rendered, @"");
    }

    #[test]
    fn renders_single_pane_with_fixed_summary_height() {
        let panes = vec![pane(&["one", "two"])];
        let rendered = preview(8, 2, panes.iter());
        insta::with_settings!({filters => vec![(r"\x1b\[0m", "<RESET>")]}, {
            insta::assert_snapshot!(rendered, @r###"
            one
            two
            <RESET>────────
            "###);
        });
    }

    #[test]
    fn renders_all_panes_and_uses_last_lines() {
        let panes = vec![pane(&["a1", "a2", "a3"]), pane(&["b1", "b2", "b3", "b4"])];
        let rendered = preview(6, 2, panes.iter());
        insta::with_settings!({filters => vec![(r"\x1b\[0m", "<RESET>")]}, {
            insta::assert_snapshot!(rendered, @r###"
            a2
            a3
            <RESET>──────
            b3
            b4
            <RESET>──────
            "###);
        });
    }

    #[test]
    fn does_not_pad_when_pane_has_less_content() {
        let panes = vec![pane(&["a"]), pane(&["b", "c", "d"])];
        let rendered = preview(5, 3, panes.iter());
        insta::with_settings!({filters => vec![(r"\x1b\[0m", "<RESET>")]}, {
            insta::assert_snapshot!(rendered, @r###"
            a
            <RESET>─────
            b
            c
            d
            <RESET>─────
            "###);
        });
    }

    #[test]
    fn trims_empty_prefix_and_suffix_before_summary() {
        let panes = vec![pane(&["", "", "a", "", ""])];
        let rendered = preview(5, 2, panes.iter());
        insta::with_settings!({filters => vec![(r"\x1b\[0m", "<RESET>")]}, {
            insta::assert_snapshot!(rendered, @r###"
            a
            <RESET>─────
            "###);
        });
    }

    #[test]
    fn skips_fully_empty_panes() {
        let panes = vec![pane(&["", " "]), pane(&["a"])];
        let rendered = preview(5, 2, panes.iter());
        insta::with_settings!({filters => vec![(r"\x1b\[0m", "<RESET>")]}, {
            insta::assert_snapshot!(rendered, @r###"
            a
            <RESET>─────
            "###);
        });
    }
}
