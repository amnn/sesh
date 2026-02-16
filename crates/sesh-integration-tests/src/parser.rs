//! Parser for markdown-driven integration test scripts.
//!
//! Each input line is preserved verbatim in the AST (`Line::raw`) and classified into a structured
//! `LineKind`. Directive parse failures become `LineKind::Error` entries so test output can show
//! parser issues in context instead of failing early.

use anyhow::Context as _;
use anyhow::bail;
use anyhow::ensure;
use regex::Regex;

/// Entrypoint for the parsed representation of a test script.
#[derive(Debug)]
pub(crate) struct Script<'s> {
    pub(crate) lines: Vec<Line<'s>>,
}

#[derive(Debug)]
pub(crate) struct Line<'s> {
    pub(crate) kind: LineKind,
    pub(crate) raw: &'s str,
}

/// Structured representation of a single line in a test script.
#[derive(Debug)]
pub(crate) enum LineKind {
    /// The line is a plain markdown/text line.
    Text,

    /// Run a host command.
    Sh { args: Vec<String> },

    /// Run a tmux command on the test socket.
    Tmux { args: Vec<String> },

    /// Set the current pane target.
    Pane { target: String },

    /// Send key inputs to current pane.
    Keys { keys: Vec<Key> },

    /// Capture pane output and apply regex replacement filters.
    Snap { filters: Vec<Filter> },

    /// The directive failed to parse.
    Error { message: String },
}

/// One key token parsed from `:keys`.
#[derive(Debug, Clone)]
pub(crate) enum Key {
    Backspace,
    Ctrl,
    Down,
    Enter,
    Esc,
    Left,
    Opt,
    Right,
    Shift,
    Space,
    Tab,
    Text(String),
    Up,
}

/// One regex replacement filter parsed from `:snap`.
#[derive(Debug)]
pub(crate) struct Filter {
    pub(crate) patt: Regex,
    pub(crate) repl: String,
}

impl<'s> Script<'s> {
    /// Parse a full script into an AST.
    pub(crate) fn parse(input: &'s str) -> Self {
        let lines = input.lines().map(Line::parse).collect();
        Self { lines }
    }
}

impl<'s> Line<'s> {
    /// Parse a source line.
    ///
    /// Lines starting with `:` are treated as directives, otherwise the line is treated as plain
    /// text. A failure to parse a command yields a `LineKind::Error` which can be rendered inline
    /// instead of failing the whole script parse.
    fn parse(raw: &'s str) -> Self {
        let Some(rest) = raw.strip_prefix(':') else {
            return Self {
                kind: LineKind::Text,
                raw,
            };
        };

        let kind = LineKind::parse(rest.trim()).unwrap_or_else(|error| LineKind::Error {
            message: format!("{error:?}"),
        });

        Self { kind, raw }
    }
}

impl LineKind {
    /// Parse one directive payload (without a leading `:`) into a `LineKind`.
    fn parse(rest: &str) -> anyhow::Result<Self> {
        let (cmd, args) = rest
            .trim()
            .split_once(char::is_whitespace)
            .unwrap_or((rest, ""));

        let Some(args) = shlex::split(args) else {
            bail!("invalid shell arguments");
        };

        Ok(match cmd {
            "s" | "sh" => LineKind::Sh { args },

            "t" | "tmux" => LineKind::Tmux { args },

            "p" | "pane" => LineKind::Pane {
                target: {
                    ensure!(args.len() == 1, "':pane' expects exactly one argument");
                    args.into_iter().next().unwrap()
                },
            },

            "k" | "keys" => LineKind::Keys {
                keys: args.into_iter().map(parse_key).collect(),
            },

            "snap" => LineKind::Snap {
                filters: args
                    .into_iter()
                    .map(parse_filter)
                    .collect::<anyhow::Result<_>>()?,
            },

            other => bail!("unknown directive ':{other}'"),
        })
    }
}

/// Parse keys to send. Recognises a set of named keys, otherwise treats the input as literal text
/// to send.
fn parse_key(input: String) -> Key {
    match input.as_str() {
        "backspace" => Key::Backspace,
        "ctrl" => Key::Ctrl,
        "down" => Key::Down,
        "enter" => Key::Enter,
        "esc" => Key::Esc,
        "left" => Key::Left,
        "opt" => Key::Opt,
        "right" => Key::Right,
        "shift" => Key::Shift,
        "space" => Key::Space,
        "tab" => Key::Tab,
        "up" => Key::Up,
        _ => Key::Text(input),
    }
}

/// Parse a filter for `:snap` output.
///
/// A filter is a regular expression pattern and replacement string, separated by a common
/// delimiter character. For example, `/foo/bar/` or `|foo|bar|` (the delimiter can be any
/// character as long as it doesn't appear in the pattern or replacement).
fn parse_filter(input: String) -> anyhow::Result<Filter> {
    let delim = input.chars().next().context("empty filter string")?;

    let input = input
        .strip_prefix(delim)
        .unwrap()
        .strip_suffix(delim)
        .context("missing closing delimiter")?;

    let mut parts = input.split(delim);
    let patt = parts.next().context("missing pattern")?;
    let repl = parts.next().context("missing replacement")?;
    ensure!(parts.next().is_none(), "trailing content after replacement");

    let patt = Regex::new(patt).context("invalid regex pattern")?;
    let repl = repl.to_owned();

    Ok(Filter { patt, repl })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mixed_text_and_directives() {
        insta::assert_debug_snapshot!(Script::parse(
            &[
                r#"# Scenario"#,
                r#""#,
                r#":s cargo --version"#,
                r#":t new-session -d -s fixture "sleep 3600""#,
                r#":p runner:0.0"#,
                r#":k down "abc""#,
                r#":snap /foo/bar/"#,
                r#""#,
                r#"Notes."#,
                r#""#,
            ]
            .join("\n")
        ));
    }

    #[test]
    fn parses_empty_script() {
        insta::assert_debug_snapshot!(Script::parse(""));
    }

    #[test]
    fn captures_unknown_directive_as_error() {
        insta::assert_debug_snapshot!(Script::parse(&[":unknown abc", ""].join("\n")));
    }

    #[test]
    fn captures_bad_shlex_as_error() {
        insta::assert_debug_snapshot!(Script::parse(&[r#":sh "unterminated"#, r#""#].join("\n")));
    }

    #[test]
    fn captures_unterminated_keys_string_as_error() {
        insta::assert_debug_snapshot!(Script::parse(&[r#":k "unterminated"#, r#""#].join("\n")));
    }

    #[test]
    fn captures_bad_filters_as_error() {
        insta::assert_debug_snapshot!(Script::parse(
            &[
                r#":snap """#,
                r#":snap /foo"#,
                r#":snap /foo/"#,
                r#":snap /foo/bar"#,
                r#":snap /foo/bar/baz/"#,
            ]
            .join("\n")
        ));
    }

    #[test]
    fn captures_invalid_snap_regex_as_error() {
        insta::assert_debug_snapshot!(Script::parse(
            &[":snap /(unterminated/repl/", ""].join("\n")
        ));
    }

    #[test]
    fn parses_snap_with_multiple_filters() {
        insta::assert_debug_snapshot!(Script::parse(
            &[":snap /foo/bar/  |baz|qux|", ""].join("\n")
        ));
    }

    #[test]
    fn parses_keys_with_escaped_quote_text() {
        insta::assert_debug_snapshot!(Script::parse(&[r#":k "a\"b""#, r#""#].join("\n")));
    }

    #[test]
    fn parses_keys_with_escaped_backslash_text() {
        insta::assert_debug_snapshot!(Script::parse(&[r#":k "a\\b""#, r#""#].join("\n")));
    }

    #[test]
    fn parses_keys_with_escaped_backslash_followed_by_escaped_quote() {
        insta::assert_debug_snapshot!(Script::parse(&[r#":k "a\\\"b""#, r#""#].join("\n")));
    }

    #[test]
    fn parses_keys_with_escaped_backslash_followed_by_quote() {
        insta::assert_debug_snapshot!(Script::parse(&[r#":k "a\\""#, r#""#].join("\n")));
    }
}
