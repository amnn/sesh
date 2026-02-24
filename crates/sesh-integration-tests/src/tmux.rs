use std::borrow::Cow;
use std::ffi::OsStr;
use std::fmt::Write;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::ensure;

use crate::env::Env;

/// Handle for managing a `tmux` server.
pub(crate) struct Tmux {
    bin: PathBuf,
    socket: PathBuf,
}

/// A `tmux` command represented as argv parts.
#[derive(Debug, Clone, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) struct Command {
    line: String,
}

impl Tmux {
    /// Start a `tmux` server in the given `env`ironment.
    ///
    /// Fails if the tmux socket file already exists, the `tmux` binary can't be added to the
    /// environment, or the server fails to start.
    pub(crate) async fn new(env: &Env) -> anyhow::Result<Self> {
        let socket = env.path("tmux.sock");
        ensure!(!socket.exists(), "tmux socket already exists");

        let bin = env.bin("tmux").await?;
        let tmux = Self { bin, socket };

        let new = tmux
            .command(env)
            .args(["new-session", "-d"])
            .args(["-x", "160", "-y", "100"])
            .output()
            .await
            .context("failed to execute 'tmux new'")?;

        ensure!(
            new.status.success(),
            "'tmux new' failed: {}",
            String::from_utf8_lossy(&new.stderr),
        );

        Ok(tmux)
    }

    /// Build a `tmux` command in the given `env`ironment.
    pub(crate) fn command(&self, env: &Env) -> tokio::process::Command {
        let mut command = env.command("tmux");
        command.arg("-S").arg(self.socket.as_os_str());
        command
    }
}

#[allow(dead_code)]
impl Command {
    /// Construct a command from the command name.
    pub(crate) fn new(name: impl AsRef<OsStr>) -> Self {
        Self {
            line: escape(name.as_ref()).into_owned(),
        }
    }

    /// Add one argument.
    pub(crate) fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.line.push(' ');
        self.line.push_str(&escape(arg.as_ref()));
        self
    }

    /// Add many arguments.
    pub(crate) fn args(mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Self {
        for arg in args {
            self.line.push(' ');
            self.line.push_str(&escape(arg.as_ref()));
        }

        self
    }
}

impl Drop for Tmux {
    fn drop(&mut self) {
        let _ = std::process::Command::new(&self.bin)
            .arg("-S")
            .arg(self.socket.as_os_str())
            .arg("kill-server")
            .status();
    }
}

/// Escape a command argument using tmux's syntax.
///
/// Empty strings are quoted, otherwise strings that are comprised of just graphics characters and
/// no characters that have a special meaning to tmux are left unquoted.
///
/// Otherwise, the string is wrapped in double quotes, with special characters escaped.
fn escape(part: &OsStr) -> Cow<'_, str> {
    let bytes = part.as_encoded_bytes();

    if bytes.is_empty() {
        return "''".into();
    }

    const SPECIAL: &[u8] = b"\"#$';\\~";
    fn needs_escape(b: &u8) -> bool {
        !b.is_ascii_graphic() || SPECIAL.contains(b)
    }

    if !bytes.iter().any(needs_escape) {
        // SAFETY: Check ensures that bytes contain a subset of ASCII graphics characters.
        return unsafe { str::from_utf8_unchecked(bytes).into() };
    }

    let mut escaped = String::with_capacity(part.len() + 2);
    escaped.push('"');

    for b in bytes {
        match *b {
            b' ' => escaped.push(' '),
            b'\n' => escaped.push_str("\\n"),
            b'\r' => escaped.push_str("\\r"),
            b'\t' => escaped.push_str("\\t"),

            b if SPECIAL.contains(&b) => {
                escaped.push('\\');
                escaped.push(b as char);
            }

            b if b.is_ascii_graphic() => escaped.push(b as char),
            b => write!(escaped, "\\{:03o}", b).unwrap(),
        }
    }

    escaped.push('"');
    escaped.into()
}

/// Decode tmux control-mode escaping in one output payload line.
///
/// This decodes octal escapes (`\ooo`) and escaped backslashes (`\\`).
#[allow(dead_code)]
pub(crate) fn unescape(line: &[u8]) -> anyhow::Result<Cow<'_, [u8]>> {
    if !line.contains(&b'\\') {
        return Ok(line.into());
    }

    fn is_octal(byte: u8) -> bool {
        matches!(byte, b'0'..=b'7')
    }

    let mut output = Vec::with_capacity(line.len());
    let mut bytes = line.iter().copied();
    while let Some(byte) = bytes.next() {
        if byte != b'\\' {
            output.push(byte);
            continue;
        }

        let e0 = bytes.next().context("unfinished escape")?;
        ensure!(is_octal(e0) || e0 == b'\\', "malformed escape");

        if e0 == b'\\' {
            output.push(b'\\');
            continue;
        }

        let e1 = bytes.next().context("unfinished escape")?;
        ensure!(is_octal(e1), "malformed escape");

        let e2 = bytes.next().context("unfinished escape")?;
        ensure!(is_octal(e2), "malformed escape");

        let byte: u16 = (e0 - b'0') as u16 * 64 + (e1 - b'0') as u16 * 8 + (e2 - b'0') as u16;
        ensure!(byte <= 0xFF, "overflow");

        output.push(byte as u8);
    }

    Ok(output.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_command_with_barewords() {
        let command = Command::new("list-sessions").arg("-F").arg("#S");
        insta::assert_snapshot!(command.line, @r###"list-sessions -F "\#S""###);
    }

    #[test]
    fn serializes_command_with_spaces_and_quotes() {
        let command = Command::new("display-message")
            .arg("-p")
            .arg("pane has spaces")
            .arg("\"quoted\"");

        insta::assert_snapshot!(command.line, @r###"display-message -p "pane has spaces" "\"quoted\"""###);
    }

    #[test]
    fn serializes_command_with_backslashes_semicolons_and_empty_parts() {
        let command = Command::new("send-keys")
            .arg("-t")
            .arg("%1")
            .arg(r"path\\segment")
            .arg(";")
            .arg("");

        insta::assert_snapshot!(command.line, @r###"send-keys -t %1 "path\\\\segment" "\;" ''"###);
    }

    #[test]
    fn unescapes_output_without_escapes_as_borrowed() {
        let input = b"plain output";
        let output = unescape(input).expect("unescape should succeed");
        assert!(matches!(output, Cow::Borrowed(_)));
        assert_eq!(output.as_ref(), input);
    }

    #[test]
    fn unescapes_octal_and_backslash_sequences() {
        let input = br"one\040two\134three\\four\073";
        let output = unescape(input).expect("unescape should succeed");
        assert_eq!(output.as_ref(), b"one two\\three\\four;");
    }

    #[test]
    fn rejects_malformed_escapes() {
        let input = br"bad\08 tail\x\";
        let error = unescape(input).expect_err("unescape should fail");
        assert_eq!(error.to_string(), "malformed escape");
    }

    #[test]
    fn rejects_non_octal_escape() {
        let input = br"bad\999x";
        let error = unescape(input).expect_err("unescape should fail");
        assert_eq!(error.to_string(), "malformed escape");
    }

    #[test]
    fn rejects_escape_overflow() {
        let input = br"bad\777";
        let error = unescape(input).expect_err("unescape should fail");
        assert_eq!(error.to_string(), "overflow");
    }

    #[test]
    fn rejects_character_escape() {
        let input = br"bad\x";
        let error = unescape(input).expect_err("unescape should fail");
        assert_eq!(error.to_string(), "malformed escape");
    }

    #[test]
    fn rejects_unfinished_escape() {
        let input = br"bad\";
        let error = unescape(input).expect_err("unescape should fail");
        assert_eq!(error.to_string(), "unfinished escape");
    }
}
