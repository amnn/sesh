// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Minimal tmux control-mode client for integration tests.

use std::borrow::Cow;
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;

use anyhow::Context as _;
use anyhow::anyhow;
use anyhow::bail;
use anyhow::ensure;
use tokio::fs;
use tokio::io::AsyncBufReadExt as _;
use tokio::io::AsyncWriteExt as _;
use tokio::io::BufReader;
use tokio::process::Child;
use tokio::process::ChildStderr;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_util::task::AbortOnDropHandle;
use tracing::debug;
use tracing::error;
use tracing::warn;

use crate::env::Env;

const TMUX_CONFIG: &str = r#"
set -g default-shell /bin/sh
set -g default-command \"/bin/sh -i\"
set -g default-size 160x100
set -g window-size manual
"#;

/// A `tmux` command represented as a single escaped line.
#[derive(Debug, Clone)]
pub(crate) struct Command {
    tx: mpsc::Sender<Request>,
    line: String,
}

/// Handle for managing a `tmux` server.
pub(crate) struct Tmux {
    pane_rx: watch::Receiver<String>,
    pane_tx: watch::Sender<String>,
    socket: PathBuf,
    _stderr_task: AbortOnDropHandle<()>,
    stdout_task: JoinHandle<()>,
    tx: mpsc::Sender<Request>,
}

/// A request to the control task to run a command and return its output.
struct Request {
    /// The command to run.
    cmd: String,

    /// A channel to receive the command's output. Output is streamed down this channel,
    /// line-by-line.
    tx: Response,
}

type Response = mpsc::Sender<anyhow::Result<Vec<u8>>>;

impl Tmux {
    /// Start a `tmux` control-mode client in the given environment.
    ///
    /// `tmux -C` will automatically spawn the server daemon on the target socket when needed.
    /// Shutdown sends `kill-server` through the same control client to tear down that daemon.
    ///
    /// Fails if the tmux socket file already exists, the `tmux` binary can't be added to the
    /// environment, or the control client fails to start.
    pub(crate) async fn new(env: &Env) -> anyhow::Result<Self> {
        // Ensure `tmux` is available in the environment.
        env.bin("tmux").await?;

        let conf = env.path("home").join(".tmux.conf");
        fs::write(conf, TMUX_CONFIG)
            .await
            .context("failed to write tmux config")?;

        let socket = env.path("tmux.sock");
        ensure!(!socket.exists(), "tmux socket already exists");

        // Start a control-mode client on a dedicated socket. tmux auto-starts the server daemon
        // for this socket if needed.
        let mut client = env
            .command("tmux")
            .args(["-C", "-S"])
            .arg(socket.as_os_str())
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to spawn control-mode tmux client")?;

        let stderr = client.stderr.take().unwrap();
        let stderr_task = AbortOnDropHandle::new(tokio::task::spawn(stderr_task(stderr)));

        let (tx, rx) = mpsc::channel(32);
        let (pane_tx, pane_rx) = watch::channel(String::new());
        let stdout_task = tokio::task::spawn(stdout_task(client, rx, pane_tx.clone()));

        let tmux = Self {
            pane_rx,
            pane_tx,
            socket,
            _stderr_task: stderr_task,
            stdout_task,
            tx,
        };

        // Set-up a subscription to track the current pane.
        tmux.command("refresh-client")
            .args(["-B", "runner-pane::#{pane_id}"])
            .status()
            .await
            .context("failed to subscribe to current pane changes")?;

        // Also update the current pane synchronously, so it has an initial value.
        tmux.refresh_pane()
            .await
            .context("failed to confirm initial current pane")?;

        Ok(tmux)
    }

    /// Create a new `Command` with the given name, that will be run against this `Tmux` instance.
    pub(crate) fn command(&self, name: impl AsRef<OsStr>) -> Command {
        Command {
            tx: self.tx.clone(),
            line: escape(name.as_ref()).into_owned(),
        }
    }

    /// Return the current pane reported by the control-mode client.
    ///
    /// `Tmux::new` confirms and seeds this value before constructing a runner, and the control
    /// task keeps it current from subscription notifications.
    pub(crate) fn pane(&self) -> String {
        self.pane_rx.borrow().clone()
    }

    /// Poll tmux for the current pane and update the cached value used for shell directives.
    pub(crate) async fn refresh_pane(&self) -> anyhow::Result<String> {
        let output = self
            .command("display-message")
            .args(["-p", "#{pane_id}"])
            .status()
            .await?;

        let output = String::from_utf8_lossy(&output);
        let pane = output.trim();
        ensure!(pane.starts_with('%'), "invalid current pane '{pane}'");

        let pane = pane.to_owned();
        self.pane_tx.send_replace(pane.clone());
        Ok(pane)
    }

    /// Gracefully shutdown the `tmux` server and control client, waiting for them to exit.
    pub(crate) async fn shutdown(self) -> anyhow::Result<()> {
        // `tmux -C` may have started the server daemon for this socket, so tear down the server
        // explicitly before dropping the control loop.
        self.command("kill-server").status().await.ok();

        drop(self.tx);
        self.stdout_task.await.ok();

        Ok(())
    }

    /// Return the path to the tmux server socket used by this runner.
    pub(crate) fn socket(&self) -> &Path {
        &self.socket
    }
}

impl Command {
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

    /// Run this command against the control-mode `tmux` instance it was created from and stream
    /// output lines.
    ///
    /// The returned channel yields one item per output line while the command is active. When tmux
    /// emits `%end`, the channel is closed and no additional item is sent. When tmux emits
    /// `%error`, a final `Err(...)` item is sent and then the channel is closed.
    pub(crate) async fn output(self) -> anyhow::Result<mpsc::Receiver<anyhow::Result<Vec<u8>>>> {
        let (tx, rx) = mpsc::channel(32);
        self.tx
            .send(Request { cmd: self.line, tx })
            .await
            .context("failed to queue tmux command")?;

        Ok(rx)
    }

    /// Run this command against the control-mode `tmux` instance it was created from and collect
    /// output.
    ///
    /// Output lines are joined with trailing newlines until the command terminates. `%end` maps to
    /// `Ok(collected_bytes)`. `%error` maps to `Err(...)`, where the error message is built from
    /// the collected bytes using lossy UTF-8 conversion.
    pub(crate) async fn status(self) -> anyhow::Result<Vec<u8>> {
        let mut joined = vec![];
        let mut output = self.output().await?;
        while let Some(line) = output.recv().await {
            match line {
                Ok(line) => {
                    joined.extend_from_slice(&line);
                    joined.push(b'\n');
                }

                Err(_) => bail!(String::from_utf8_lossy(&joined).into_owned()),
            }
        }

        Ok(joined)
    }
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

/// Task looking after `stdout` (and `stdin`) for a control-mode `tmux` client.
///
/// `client` is kept alive by this task, and `requests` receives control commands to send to tmux.
async fn stdout_task(
    mut client: Child,
    mut requests: mpsc::Receiver<Request>,
    pane: watch::Sender<String>,
) {
    let Some(mut stdin) = client.stdin.take() else {
        error!("failed to setup control client stdin");
        return;
    };

    let Some(stdout) = client.stdout.take() else {
        error!("failed to setup control client stdout");
        return;
    };
    let mut stdout = BufReader::new(stdout);
    let mut line = Vec::new();

    let mut active: Option<Response> = None;
    let mut pending: VecDeque<Response> = VecDeque::new();

    // The control client emits a `%begin`/`%end` pair associated with the command that spins up
    // the control mode client. Add a channel to `pending` (already closed) to soak up the output
    // of this command, so future commands remain properly aligned with the control client's
    // output.
    pending.push_back(mpsc::channel(1).0);

    loop {
        tokio::select! {
            request = requests.recv() => {
                let Some(request) = request else {
                    debug!("requests channel closed, shutting down...");
                    break;
                };

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

            read = stdout.read_until(b'\n', &mut line) => {
                let read = match read {
                    Ok(read) => read,

                    Err(e) => {
                        error!("stdout error: {e}");
                        break;
                    }
                };

                if read == 0 {
                    warn!("stdout closed");
                    break;
                }

                if line.ends_with(b"\n") {
                    line.pop();
                }

                if line.ends_with(b"\r") {
                    line.pop();
                }

                if let Some(tx) = &active && line.starts_with(b"%error") {
                    let _ = tx.send(Err(anyhow!("tmux command failed"))).await;
                    active = None;
                } else if active.is_some() && line.starts_with(b"%end") {
                    active = None;
                } else if line.starts_with(b"%output ") {
                    debug!("notification: {}", String::from_utf8_lossy(&line));
                } else if let Some(tx) = &active {
                    let _ = tx.send(unescape(line.clone())).await;
                } else if line.starts_with(b"%begin") {
                    active = pending.pop_front();
                } else if let Some(rest) = line.strip_prefix(b"%subscription-changed runner-pane ")
                    && let Some(rest) = str::from_utf8(rest).ok()
                    && let Some((_, p)) = rest.rsplit_once(" : ")
                    && p.starts_with('%')
                {
                    pane.send_replace(p.to_owned());
                } else if line.starts_with(b"%") {
                    debug!("notification: {}", String::from_utf8_lossy(&line));
                } else {
                    warn!("unexpected: {}", String::from_utf8_lossy(&line));
                }

                line.clear();
            }
        }
    }

    // If the task is exiting while there are still pending tasks, unblock them by sending an error
    // down their channels.
    for tx in active.into_iter().chain(pending) {
        let _ = tx.send(Err(anyhow!("unexpected exit"))).await;
    }

    // Kill the control client now that command processing has stopped.
    client.kill().await.ok();
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn escapes_backslashes() {
        let escaped = escape(OsStr::new(r"path\\segment"));
        insta::assert_snapshot!(escaped.as_ref(), @r###""path\\\\segment""###);
    }

    #[test]
    fn escapes_barewords() {
        let escaped = escape(OsStr::new("list-sessions"));
        insta::assert_snapshot!(escaped.as_ref(), @"list-sessions");
    }

    #[test]
    fn escapes_spaces() {
        let escaped = escape(OsStr::new("pane has spaces"));
        insta::assert_snapshot!(escaped.as_ref(), @r###""pane has spaces""###);
    }

    #[test]
    fn rejects_character_escape() {
        let input = br"bad\x".to_vec();
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
    fn rejects_unfinished_escape() {
        let input = br"bad\".to_vec();
        let error = unescape(input).expect_err("unescape should fail");
        assert_eq!(error.to_string(), "unfinished escape");
    }

    #[test]
    fn unescapes_octal_and_backslash_sequences() {
        let input = br"one\040two\134three\\four\073".to_vec();
        let output = unescape(input).expect("unescape should succeed");
        assert_eq!(output, b"one two\\three\\four;");
    }

    #[test]
    fn unescapes_output_without_escapes_is_unchanged() {
        let input = b"plain output".to_vec();
        let output = unescape(input.clone()).expect("unescape should succeed");
        assert_eq!(output, input);
    }
}
