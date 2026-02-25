use std::borrow::Cow;
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fmt::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::anyhow;
use anyhow::ensure;
use tokio::io::AsyncBufReadExt as _;
use tokio::io::AsyncWriteExt as _;
use tokio::io::BufReader;
use tokio::process::Child;
use tokio::process::ChildStderr;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_util::task::AbortOnDropHandle;
use tracing::debug;
use tracing::error;
use tracing::warn;

use crate::env::Env;

/// Handle for managing a `tmux` server.
pub(crate) struct Tmux {
    _stderr_task: AbortOnDropHandle<()>,
    _stdout_task: AbortOnDropHandle<()>,
    _tx: mpsc::Sender<Request>,
    socket: PathBuf,
}

/// A `tmux` command represented as a single line of text, with proper escaping.
#[derive(Debug, Clone, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) struct Command {
    line: String,
}

type Response = mpsc::Sender<anyhow::Result<Vec<u8>>>;

/// A request to the control task to run a command and return its output.
struct Request {
    /// The command to run.
    cmd: String,

    /// A channel to receive the command's output. Output is streamed down this channel,
    /// line-by-line.
    tx: Response,
}

impl Tmux {
    /// Start a `tmux` server in the given `env`ironment.
    ///
    /// Fails if the tmux socket file already exists, the `tmux` binary can't be added to the
    /// environment, or the server fails to start.
    pub(crate) async fn new(env: &Env) -> anyhow::Result<Self> {
        // Ensure `tmux` is available in the environment.
        env.bin("tmux").await?;

        let socket = env.path("tmux.sock");
        ensure!(!socket.exists(), "tmux socket already exists");

        // Start tmux in control mode, and not in daemon mode, with a dedicated socket.
        //
        // This results in a single long-lived tmux process that is both the server and the client.
        // When the client process exits, the tmux server will exit with it. This can be triggered
        // by dropping the child handle.
        //
        // Commands can be sent to the tmux server via `stdin` of this process, while the server
        // communicates notifications and command outputs via `stdout`. The server can also be
        // interacted with by other `tmux` commands, using the same socket.
        let mut child = env
            .command("tmux")
            .args(["-D", "-C", "-S"])
            .arg(socket.as_os_str())
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to spawn control-mode tmux client")?;

        let stderr = child.stderr.take().unwrap();
        let stderr_task = AbortOnDropHandle::new(tokio::task::spawn(stderr_task(stderr)));

        let (_tx, rx) = mpsc::channel(32);
        let stdout_task = AbortOnDropHandle::new(tokio::task::spawn(stdout_task(child, rx)));

        wait_until_ready(env, &socket).await?;

        // TODO: Send down channel, eventually.
        let new = env
            .command("tmux")
            .arg("-S")
            .arg(socket.as_os_str())
            .args(["new-session", "-d", "-x", "160", "-y", "100"])
            .output()
            .await
            .context("failed to execute 'tmux new-session'")?;

        ensure!(
            new.status.success(),
            "'tmux new-session' failed: {}",
            String::from_utf8_lossy(&new.stderr),
        );

        Ok(Self {
            _stderr_task: stderr_task,
            _stdout_task: stdout_task,
            _tx,
            socket,
        })
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

    /// Run this command against a control-mode `tmux` instance and stream output lines.
    ///
    /// The returned channel yields one item per output line while the command is active. When tmux
    /// emits `%end`, the channel is closed and no additional item is sent. When tmux emits
    /// `%error`, a final `Err(...)` item is sent and then the channel is closed.
    pub(crate) async fn output(
        self,
        tmux: &Tmux,
    ) -> anyhow::Result<mpsc::Receiver<anyhow::Result<Vec<u8>>>> {
        let (tx, rx) = mpsc::channel(32);
        tmux._tx
            .send(Request { cmd: self.line, tx })
            .await
            .context("failed to queue tmux command")?;

        Ok(rx)
    }

    /// Run this command against a control-mode `tmux` instance and collect output.
    ///
    /// Output lines are joined with trailing newlines until the command terminates. `%end` maps to
    /// `Ok(collected_bytes)`. `%error` maps to `Err(...)`, where the error message is built from
    /// the collected bytes using lossy UTF-8 conversion.
    pub(crate) async fn status(self, tmux: &Tmux) -> anyhow::Result<Vec<u8>> {
        let mut output = self.output(tmux).await?;

        let mut joined = vec![];
        while let Some(line) = output.recv().await {
            match line {
                Ok(line) => {
                    joined.extend_from_slice(&line);
                    joined.push(b'\n');
                }

                Err(e) => {
                    let output = String::from_utf8_lossy(&joined);
                    return Err(anyhow!("tmux command failed: {e:?}\noutput:\n{output}"));
                }
            }
        }

        Ok(joined)
    }
}

/// Task looking after `stdout` (and `stdin`) for a control-mode `tmux` client.
///
/// `child` is a handle to the child process that is kept alive by this task, while `requests` is a
/// channel on which the task receives requests to pass on to the `tmux`. Each request includes a
/// command to run, and a channel on which to send the command's output back to the requester.
async fn stdout_task(mut child: Child, mut requests: mpsc::Receiver<Request>) {
    let Some(mut stdin) = child.stdin.take() else {
        error!("failed to setup stdin");
        return;
    };

    let Some(mut stdout) = child.stdout.take().map(|p| BufReader::new(p).lines()) else {
        error!("failed to setup stdout");
        return;
    };

    let mut active: Option<Response> = None;
    let mut pending: VecDeque<Response> = VecDeque::new();

    loop {
        tokio::select! {
            Some(request) = requests.recv() => {
                pending.push_back(request.tx);

                if let Err(e) = async {
                    stdin.write_all(request.cmd.as_bytes()).await?;
                    stdin.write_all(b"\n").await?;
                    stdin.flush().await
                }.await {
                    // SAFETY: `request.tx` was pushed to the back of `pending` at the top of this
                    // select arm, and nothing has accessed `pending` since then.
                    let tx = pending.pop_back().unwrap();
                    let _ = tx.send(Err(anyhow!(e).context("failed to send command"))).await;
                }
            }

            line = stdout.next_line() => {
                let line = match line {
                    Ok(Some(line)) => line,

                    Ok(None) => {
                        warn!("stdout closed");
                        break;
                    }

                    Err(e) => {
                        error!("stdout error: {e}");
                        break;
                    }
                };

                if let Some(tx) = &active && line.starts_with("%error") {
                    let _ = tx.send(Err(anyhow!("tmux command failed"))).await;
                    active = None;
                } else if active.is_some() && line.starts_with("%end") {
                    active = None;
                } else if let Some(tx) = &active {
                    let _ = tx.send(unescape(line.into_bytes())).await;
                } else if line.starts_with("%begin") {
                    active = pending.pop_front();
                } else if line.starts_with("%") {
                    debug!("notification: {line}");
                } else {
                    warn!("unexpected: {line}");
                }
            }
        }
    }

    // If the task is exiting while there are still pending tasks, unblock them by sending an error
    // down their channels.
    if let Some(tx) = active {
        let _ = tx.send(Err(anyhow!("unexpected exit"))).await;
    }

    while let Some(tx) = pending.pop_front() {
        let _ = tx.send(Err(anyhow!("unexpected exit"))).await;
    }
}

/// A task to monitor `stderr` for a control-mode `tmux` client, and log any output as error
/// traces.
async fn stderr_task(stderr: ChildStderr) {
    let mut stderr = BufReader::new(stderr).lines();

    loop {
        match stderr.next_line().await {
            Ok(Some(line)) => error!("stderr: {line}"),

            Ok(None) => {
                warn!("stderr closed");
                break;
            }

            Err(e) => {
                error!("stderr error: {e}");
                break;
            }
        }
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
pub(crate) fn unescape(line: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    if !line.contains(&b'\\') {
        return Ok(line);
    }

    fn is_octal(byte: u8) -> bool {
        matches!(byte, b'0'..=b'7')
    }

    let mut output = Vec::with_capacity(line.len());
    let mut bytes = line.into_iter();
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

    Ok(output)
}

// TODO: Temporary measure while we're in a transition period where some command runs are via the
// control task and some are direct spawns. We should eventually move all command runs to the
// control task and remove this.
async fn wait_until_ready(env: &Env, socket: &Path) -> anyhow::Result<()> {
    let mut command = env.command("tmux");
    command
        .arg("-S")
        .arg(socket.as_os_str())
        .args(["display-message", "-p", "#{version}"]);

    for _ in 0..200 {
        if socket.exists()
            && let Ok(output) = command.output().await
            && output.status.success()
        {
            return Ok(());
        }

        sleep(Duration::from_millis(10)).await;
    }

    ensure!(socket.exists(), "tmux socket was not created");
    anyhow::bail!("tmux control-mode client did not become ready")
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
    fn unescapes_output_without_escapes_is_unchanged() {
        let input = b"plain output".to_vec();
        let output = unescape(input.clone()).expect("unescape should succeed");
        assert_eq!(output, input);
    }

    #[test]
    fn unescapes_octal_and_backslash_sequences() {
        let input = br"one\040two\134three\\four\073".to_vec();
        let output = unescape(input).expect("unescape should succeed");
        assert_eq!(output, b"one two\\three\\four;");
    }

    #[test]
    fn rejects_malformed_escapes() {
        let input = br"bad\08 tail\x\".to_vec();
        let error = unescape(input).expect_err("unescape should fail");
        assert_eq!(error.to_string(), "malformed escape");
    }

    #[test]
    fn rejects_non_octal_escape() {
        let input = br"bad\999x".to_vec();
        let error = unescape(input).expect_err("unescape should fail");
        assert_eq!(error.to_string(), "malformed escape");
    }

    #[test]
    fn rejects_escape_overflow() {
        let input = br"bad\777".to_vec();
        let error = unescape(input).expect_err("unescape should fail");
        assert_eq!(error.to_string(), "overflow");
    }

    #[test]
    fn rejects_character_escape() {
        let input = br"bad\x".to_vec();
        let error = unescape(input).expect_err("unescape should fail");
        assert_eq!(error.to_string(), "malformed escape");
    }

    #[test]
    fn rejects_unfinished_escape() {
        let input = br"bad\".to_vec();
        let error = unescape(input).expect_err("unescape should fail");
        assert_eq!(error.to_string(), "unfinished escape");
    }
}
