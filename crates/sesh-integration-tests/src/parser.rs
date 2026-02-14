//! Parser for markdown-driven integration test scripts.
//!
//! Each input line is preserved verbatim in the AST (`Line::raw`) and classified into a structured
//! `LineKind`. Directive parse failures become `LineKind::Error` entries so test output can show
//! parser issues in context instead of failing early.

use std::borrow::Cow;

use anyhow::Context as _;
use anyhow::anyhow;
use anyhow::bail;
use regex::Regex;
use winnow::Parser;
use winnow::ascii::multispace0;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::repeat;
use winnow::combinator::terminated;
use winnow::error::ErrMode;
use winnow::error::FromExternalError;
use winnow::stream::Stream;
use winnow::token::any;
use winnow::token::take_till;

type ParseError = ErrMode<winnow::error::ContextError>;

/// Entrypoint for the parsed representation of a test script.
#[derive(Debug)]
pub(crate) struct Script<'s> {
    pub(crate) lines: Vec<Line<'s>>,
}

#[derive(Debug)]
pub(crate) struct Line<'s> {
    pub(crate) kind: LineKind<'s>,
    pub(crate) raw: &'s str,
}

/// Structured representation of a single line in a test script.
#[derive(Debug)]
pub(crate) enum LineKind<'s> {
    /// The line is a plain markdown/text line.
    Text,

    /// Run a host command.
    Sh { args: Vec<String> },

    /// Run a tmux command on the test socket.
    Tmux { args: Vec<String> },

    /// Set the current pane target.
    Pane { target: &'s str },

    /// Send key inputs to current pane.
    Keys { keys: Vec<Key<'s>> },

    /// Capture pane output and apply regex replacement filters.
    Snap { filters: Vec<Filter<'s>> },

    /// The directive failed to parse.
    Error { message: String },
}

/// One key token parsed from `:keys`.
#[derive(Debug, Clone)]
pub(crate) enum Key<'s> {
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
    Text(Cow<'s, str>),
    Up,
}

/// One regex replacement filter parsed from `:snap`.
#[derive(Debug)]
pub(crate) struct Filter<'s> {
    pub(crate) patt: Regex,
    pub(crate) repl: &'s str,
}

impl<'s> Script<'s> {
    /// Parse a full script into an AST.
    pub(crate) fn parse(input: &'s str) -> Self {
        let mut lines = Vec::new();

        for line in input.lines() {
            lines.push(Line::parse(line));
        }

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

impl<'s> LineKind<'s> {
    /// Parse one directive payload (without a leading `:`) into a `LineKind`.
    fn parse(rest: &'s str) -> anyhow::Result<Self> {
        let Some((cmd, args)) = rest.split_once(char::is_whitespace) else {
            bail!("empty directive");
        };

        Ok(match cmd {
            "s" | "sh" => LineKind::Sh {
                args: shlex::split(args).context("bad shell arguments")?,
            },
            "t" | "tmux" => LineKind::Tmux {
                args: shlex::split(args).context("bad tmux arguments")?,
            },
            "p" | "pane" => LineKind::Pane {
                target: args.trim(),
            },
            "k" | "keys" => LineKind::Keys {
                keys: parse_keys(args.trim())?,
            },
            "snap" => LineKind::Snap {
                filters: parse_filters(args.trim())?,
            },
            other => bail!("unknown directive ':{other}'"),
        })
    }
}

/// Parse key arguments from a `:keys` directive.
fn parse_keys(keys: &str) -> anyhow::Result<Vec<Key<'_>>> {
    repeat(0.., terminated(parse_key, multispace0))
        .parse(keys)
        .map_err(|error| anyhow!("error parsing keys: {error}"))
}

/// Parse filter arguments from a `:snap` directive.
fn parse_filters(filters: &str) -> anyhow::Result<Vec<Filter<'_>>> {
    repeat(0.., terminated(parse_filter, multispace0))
        .parse(filters)
        .map_err(|error| anyhow!("error parsing filters: {error}"))
}

fn parse_key<'s>(input: &mut &'s str) -> Result<Key<'s>, ParseError> {
    alt((
        "backspace".value(Key::Backspace),
        "ctrl".value(Key::Ctrl),
        "down".value(Key::Down),
        "enter".value(Key::Enter),
        "esc".value(Key::Esc),
        "left".value(Key::Left),
        "opt".value(Key::Opt),
        "right".value(Key::Right),
        "shift".value(Key::Shift),
        "space".value(Key::Space),
        "tab".value(Key::Tab),
        "up".value(Key::Up),
        parse_string.map(Key::Text),
    ))
    .parse_next(input)
}

fn parse_string<'s>(input: &mut &'s str) -> Result<Cow<'s, str>, ParseError> {
    let fragments: Vec<Cow<'s, str>> =
        delimited('"', repeat(0.., parse_fragment), '"').parse_next(input)?;
    let mut iter = fragments.into_iter();

    let Some(mut result) = iter.next() else {
        return Ok(Cow::Borrowed(""));
    };

    for fragment in iter {
        result += fragment;
    }

    Ok(result)
}

fn parse_fragment<'s>(input: &mut &'s str) -> Result<Cow<'s, str>, ParseError> {
    alt((
        "\\\\".value(Cow::Borrowed("\\")),
        "\\\"".value(Cow::Borrowed("\"")),
        take_till(1.., |ch: char| ch == '"' || ch == '\\').map(Cow::Borrowed),
    ))
    .parse_next(input)
}

fn parse_filter<'s>(input: &mut &'s str) -> Result<Filter<'s>, ParseError> {
    let start = input.checkpoint();
    parse_filter_(input).inspect_err(|_| {
        input.reset(&start);
    })
}

fn parse_filter_<'s>(input: &mut &'s str) -> Result<Filter<'s>, ParseError> {
    let mut separator = any.parse_next(input)?;

    let pattern = take_till(0.., separator).parse_next(input)?;
    separator.parse_next(input)?;

    let pattern =
        Regex::new(pattern).map_err(|error| ParseError::from_external_error(input, error))?;

    let replacement = take_till(0.., separator).parse_next(input)?;
    separator.parse_next(input)?;

    Ok(Filter {
        patt: pattern,
        repl: replacement,
    })
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
    fn captures_command_without_arguments_as_error() {
        insta::assert_debug_snapshot!(Script::parse(&[r#":sh"#, r#""#].join("\n")));
    }

    #[test]
    fn captures_invalid_key_as_error() {
        insta::assert_debug_snapshot!(Script::parse(&[":k INVALID", ""].join("\n")));
    }

    #[test]
    fn captures_unterminated_keys_string_as_error() {
        insta::assert_debug_snapshot!(Script::parse(&[r#":k "unterminated"#, r#""#].join("\n")));
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
            &[":snap /foo/bar/  #baz#qux#", ""].join("\n")
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
