// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Parser for markdown-driven integration test scripts.
//!
//! Each input line is preserved verbatim in the AST (`Line::raw`) and classified into a structured
//! `LineKind`. Directive parse failures become `LineKind::Error` entries so test output can show
//! parser issues in context instead of failing early.

use std::borrow::Cow;
use std::fmt;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::bail;
use anyhow::ensure;
use clap::Parser as _;
use nonempty::NonEmpty;
use regex::Regex;
use unicode_segmentation::UnicodeSegmentation;

/// Entrypoint for the parsed representation of a test script.
#[derive(Debug)]
pub struct Script<'s> {
    pub(crate) lines: Vec<Line<'s>>,
}

/// One regex replacement filter parsed from `:snap`.
#[derive(Debug)]
pub(crate) struct Filter {
    pub(crate) patt: Regex,
    pub(crate) paint: String,
}

/// A key accompanied by optional modifiers.
#[derive(Debug, Clone)]
pub(crate) struct Key {
    pub(crate) kind: KeyKind,
    pub(crate) ctrl: bool,
    pub(crate) meta: bool,
    pub(crate) shft: bool,
}

/// One key token parsed from `:keys`.
#[derive(Debug, Clone)]
pub(crate) enum KeyKind {
    Backspace,
    Btab,
    Down,
    Enter,
    Esc,
    Left,
    Right,
    Space,
    Tab,
    Text(String),
    Up,
}

/// Source line preserved alongside its parsed classification.
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

    /// Require particular binaries be made available in the test environment.
    Bins { args: Vec<String> },

    /// Run a host command.
    Sh { args: NonEmpty<String> },

    /// Write a file beneath the test home directory from the following fenced block.
    Write { path: PathBuf },

    /// Copy a manifest-relative file into the test home directory.
    Copy { source: PathBuf, path: PathBuf },

    /// Run a tmux command on the test socket.
    Tmux { args: NonEmpty<String> },

    /// Set the current pane target.
    Pane { target: String },

    /// Send key inputs to current pane.
    Keys { keys: Vec<Key> },

    /// Capture pane output and apply regex replacement filters.
    Snap {
        count: NonZeroUsize,
        duration: Duration,
        filters: Vec<Filter>,
    },

    /// The directive failed to parse.
    Error { message: String },
}

/// Parsed arguments for `:snap`.
#[derive(clap::Parser)]
struct SnapArgs {
    /// Number of consecutive identical samples required before a snap settles.
    #[arg(short = 'c', long = "count", default_value = "5")]
    count: NonZeroUsize,

    /// Maximum time to wait for pane output to settle.
    #[arg(
        short = 'd',
        long = "duration",
        default_value = "1s",
        value_parser = parse_duration
    )]
    duration: Duration,

    /// Regex replacement filters applied to each captured pane sample.
    #[arg(value_name = "FILTER")]
    filters: Vec<String>,
}

impl<'s> Script<'s> {
    /// Parse a full script into an AST.
    pub fn parse(input: &'s str) -> Self {
        let lines = input.lines().map(Line::parse).collect();
        Self { lines }
    }
}

impl Key {
    /// Return the tmux key code for this key and its modifiers.
    pub(crate) fn code(&self) -> Cow<'_, str> {
        let code = match &self.kind {
            KeyKind::Backspace => Cow::Borrowed("BSpace"),
            KeyKind::Btab => Cow::Borrowed("BTab"),
            KeyKind::Down => Cow::Borrowed("Down"),
            KeyKind::Enter => Cow::Borrowed("Enter"),
            KeyKind::Esc => Cow::Borrowed("Escape"),
            KeyKind::Left => Cow::Borrowed("Left"),
            KeyKind::Right => Cow::Borrowed("Right"),
            KeyKind::Space => Cow::Borrowed("Space"),
            KeyKind::Tab => Cow::Borrowed("Tab"),
            KeyKind::Text(text) => Cow::Borrowed(text.as_str()),
            KeyKind::Up => Cow::Borrowed("Up"),
        };

        if !self.ctrl && !self.meta && !self.shft {
            return code;
        }

        let mut prefixed = String::new();

        if self.ctrl {
            prefixed.push_str("C-");
        }

        if self.meta {
            prefixed.push_str("M-");
        }

        if self.shft {
            prefixed.push_str("S-");
        }

        prefixed.push_str(&code);
        Cow::Owned(prefixed)
    }
}

impl<'s> Line<'s> {
    /// Parse a source line.
    ///
    /// Lines starting with `:` are treated as directives, otherwise the line is treated as plain
    /// text. A failure to parse a command yields a `LineKind::Error` which can be rendered inline
    /// instead of failing the whole script parse.
    fn parse(raw: &'s str) -> Self {
        let Some(rest) = raw
            .strip_prefix("    :")
            .or_else(|| raw.strip_prefix("\t:"))
        else {
            return Self {
                kind: LineKind::Text,
                raw,
            };
        };

        let kind = LineKind::parse(rest.trim()).unwrap_or_else(|error| LineKind::Error {
            message: format!("{error:#}"),
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
            "b" | "bins" => LineKind::Bins { args },

            "$" | "sh" => LineKind::Sh {
                args: NonEmpty::from_vec(args).context("':sh' expects at least one argument")?,
            },

            "w" | "write" => LineKind::Write {
                path: {
                    ensure!(args.len() == 1, "':write' expects exactly one argument");
                    PathBuf::from(args.into_iter().next().unwrap())
                },
            },

            "cp" | "copy" => LineKind::Copy {
                source: {
                    ensure!(args.len() == 2, "':copy' expects exactly two arguments");
                    PathBuf::from(args[0].clone())
                },
                path: PathBuf::from(args[1].clone()),
            },

            "t" | "tmux" => LineKind::Tmux {
                args: NonEmpty::from_vec(args).context("':tmux' expects at least one argument")?,
            },

            "p" | "pane" => LineKind::Pane {
                target: {
                    ensure!(args.len() == 1, "':pane' expects exactly one argument");
                    args.into_iter().next().unwrap()
                },
            },

            "k" | "keys" => {
                let keys: anyhow::Result<Vec<_>> = args.into_iter().map(parse_key).collect();
                LineKind::Keys { keys: keys? }
            }

            "s" | "snap" => {
                let args = parse_snap_args(args)?;
                let filters: anyhow::Result<Vec<_>> =
                    args.filters.into_iter().map(parse_filter).collect();
                LineKind::Snap {
                    count: args.count,
                    duration: args.duration,
                    filters: filters?,
                }
            }

            other => bail!("unknown directive ':{other}'"),
        })
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let KeyKind::Text(text) = &self.kind
            && !self.ctrl
            && !self.meta
            && !self.shft
        {
            return write!(f, "text '{text}'");
        }

        f.write_str("code '")?;

        if self.ctrl {
            f.write_str("C-")?;
        }

        if self.meta {
            f.write_str("M-")?;
        }

        if self.shft {
            f.write_str("S-")?;
        }

        match &self.kind {
            KeyKind::Backspace => f.write_str("backspace"),
            KeyKind::Btab => f.write_str("btab"),
            KeyKind::Down => f.write_str("down"),
            KeyKind::Enter => f.write_str("enter"),
            KeyKind::Esc => f.write_str("esc"),
            KeyKind::Left => f.write_str("left"),
            KeyKind::Right => f.write_str("right"),
            KeyKind::Space => f.write_str("space"),
            KeyKind::Tab => f.write_str("tab"),
            KeyKind::Text(text) => f.write_str(text),
            KeyKind::Up => f.write_str("up"),
        }?;

        f.write_str("'")
    }
}

/// Parse a human-readable duration accepted by `humantime`.
fn parse_duration(input: &str) -> anyhow::Result<Duration> {
    humantime::parse_duration(input).context("invalid duration")
}

/// Parse a filter for `:snap` output.
///
/// A filter is a regular expression pattern and a paint grapheme, separated by
/// a common delimiter character. For example, `/foo/x` or `|foo|x`.
///
/// If the pattern has capture groups, only captured group contents are painted.
fn parse_filter(input: String) -> anyhow::Result<Filter> {
    let mut chars = input.chars();
    let delim = chars.next().context("empty filter string")?;
    let rest = chars.as_str();

    let Some((patt, repl)) = rest.split_once(delim) else {
        bail!("missing separator delimiter");
    };

    ensure!(!patt.is_empty(), "missing pattern");

    let mut gs = repl.graphemes(true);
    let paint = gs.next().context("missing replacement grapheme cluster")?;
    ensure!(gs.next().is_none(), "replacement must be one grapheme");

    let patt = Regex::new(patt).context("invalid regex pattern")?;
    Ok(Filter {
        patt,
        paint: paint.to_owned(),
    })
}

/// Parse a key to send (modifiers and the key code).
fn parse_key(input: String) -> anyhow::Result<Key> {
    let mut input = input.as_str();
    let mut ctrl = false;
    let mut meta = false;
    let mut shft = false;

    loop {
        if let Some(rest) = input.strip_prefix("C-") {
            ctrl = true;
            input = rest;
        } else if let Some(rest) = input.strip_prefix("M-") {
            meta = true;
            input = rest;
        } else if let Some(rest) = input.strip_prefix("S-") {
            shft = true;
            input = rest;
        } else {
            break;
        }
    }

    let kind = parse_key_kind(input);
    if shft {
        use KeyKind as K;
        ensure!(
            matches!(kind, K::Down | K::Left | K::Right | K::Up),
            "'S-' only applies to arrow keys"
        );
    }

    if let KeyKind::Text(t) = &kind
        && (ctrl || meta)
    {
        ensure!(
            t.len() == 1 && t.is_ascii(),
            "modifiers only apply to single ASCII key codes"
        );
    }

    Ok(Key {
        kind,
        ctrl,
        meta,
        shft,
    })
}

/// Parse a key kind.
///
/// Recognises a set of named keys, otherwise treats the input as literal text to
/// send.
fn parse_key_kind(input: &str) -> KeyKind {
    match input {
        "backspace" => KeyKind::Backspace,
        "btab" => KeyKind::Btab,
        "down" => KeyKind::Down,
        "enter" => KeyKind::Enter,
        "esc" => KeyKind::Esc,
        "left" => KeyKind::Left,
        "right" => KeyKind::Right,
        "space" => KeyKind::Space,
        "tab" => KeyKind::Tab,
        "up" => KeyKind::Up,
        _ => KeyKind::Text(input.to_owned()),
    }
}

/// Parse `:snap` arguments with Clap so diagnostics match real CLI parsing.
fn parse_snap_args(args: Vec<String>) -> anyhow::Result<SnapArgs> {
    let argv = std::iter::once(":snap".to_owned()).chain(args);
    SnapArgs::try_parse_from(argv).map_err(|error| anyhow::anyhow!(error.render().to_string()))
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
                r#"    :$ cargo --version"#,
                r#"    :w tmp/hello.txt"#,
                r#"```text"#,
                r#"hello"#,
                r#"```"#,
                r#"    :cp fixtures/source.txt tmp/source.txt"#,
                r#"    :t new-session -d -s fixture "sleep 3600""#,
                r#"    :p runner:0.0"#,
                r#"    :k down "abc""#,
                r#"    :s /foo/x"#,
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
        insta::assert_debug_snapshot!(Script::parse(&["    :unknown abc", ""].join("\n")));
    }

    #[test]
    fn captures_bad_shlex_as_error() {
        insta::assert_debug_snapshot!(Script::parse(
            &[r#"    :sh "unterminated"#, r#""#].join("\n")
        ));
    }

    #[test]
    fn captures_unterminated_keys_string_as_error() {
        insta::assert_debug_snapshot!(Script::parse(
            &[r#"    :k "unterminated"#, r#""#].join("\n")
        ));
    }

    #[test]
    fn captures_bad_filters_as_error() {
        insta::assert_debug_snapshot!(Script::parse(
            &[
                r#"    :snap """#,
                r#"    :snap /foo"#,
                r#"    :snap /foo/x/"#,
                r#"    :snap /foo/bar"#,
                r#"    :snap /foo/"#,
            ]
            .join("\n")
        ));
    }

    #[test]
    fn captures_invalid_snap_regex_as_error() {
        insta::assert_debug_snapshot!(Script::parse(
            &["    :snap /(unterminated/x", ""].join("\n")
        ));
    }

    #[test]
    fn parses_snap_with_multiple_filters() {
        insta::assert_debug_snapshot!(Script::parse(&["    :snap /foo/x  |baz|q", ""].join("\n")));
    }

    #[test]
    fn parses_snap_with_multiple_capture_groups() {
        insta::assert_debug_snapshot!(Script::parse(&["    :snap /(a)(b)/x", ""].join("\n")));
    }

    #[test]
    fn parses_snap_with_grapheme_replacement() {
        insta::assert_debug_snapshot!(Script::parse(&["    :snap /foo/👩🏽‍💻", ""].join("\n")));
    }

    #[test]
    fn parses_snap_with_duration_flag() {
        insta::assert_debug_snapshot!(Script::parse(
            &["    :snap --duration 100ms /foo/x", ""].join("\n")
        ));
    }

    #[test]
    fn parses_snap_with_count_flag() {
        insta::assert_debug_snapshot!(Script::parse(
            &["    :snap --count 1 /foo/x", ""].join("\n")
        ));
    }

    #[test]
    fn captures_invalid_snap_duration_as_error() {
        insta::assert_debug_snapshot!(Script::parse(&["    :snap --duration soon", ""].join("\n")));
    }

    #[test]
    fn captures_invalid_snap_count_as_error() {
        insta::assert_debug_snapshot!(Script::parse(&["    :snap --count 0", ""].join("\n")));
    }

    #[test]
    fn parses_keys_with_escaped_quote_text() {
        insta::assert_debug_snapshot!(Script::parse(&[r#"    :k "a\"b""#, r#""#].join("\n")));
    }

    #[test]
    fn parses_keys_with_escaped_backslash_text() {
        insta::assert_debug_snapshot!(Script::parse(&[r#"    :k "a\\b""#, r#""#].join("\n")));
    }

    #[test]
    fn parses_keys_with_escaped_backslash_followed_by_escaped_quote() {
        insta::assert_debug_snapshot!(Script::parse(&[r#"    :k "a\\\"b""#, r#""#].join("\n")));
    }

    #[test]
    fn parses_keys_with_escaped_backslash_followed_by_quote() {
        insta::assert_debug_snapshot!(Script::parse(&[r#"    :k "a\\""#, r#""#].join("\n")));
    }

    #[test]
    fn parses_keys_with_single_modifier() {
        insta::assert_debug_snapshot!(Script::parse(&[r#"    :k C-a"#, r#""#].join("\n")));
    }

    #[test]
    fn parses_keys_with_stacked_modifiers() {
        insta::assert_debug_snapshot!(Script::parse(&[r#"    :k C-M-S-up"#, r#""#].join("\n")));
    }

    #[test]
    fn captures_invalid_modified_text_keys_as_error() {
        insta::assert_debug_snapshot!(Script::parse(
            &[r#"    :k C-ab"#, r#"    :k M-é"#, r#"    :k S-"""#, r#""#,].join("\n")
        ));
    }

    #[test]
    fn parses_write_directive() {
        insta::assert_debug_snapshot!(Script::parse(&["    :write tmp/hello.txt", ""].join("\n")));
    }

    #[test]
    fn parses_copy_directive() {
        insta::assert_debug_snapshot!(Script::parse(
            &["    :copy scripts/tmcap tmp/tmcap", ""].join("\n")
        ));
    }

    #[test]
    fn leaves_unindented_colon_lines_as_text() {
        insta::assert_debug_snapshot!(Script::parse(&[":sh echo nope", ""].join("\n")));
    }
}
