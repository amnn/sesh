// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Runtime for parsed markdown integration scripts.

use std::ffi::OsStr;
use std::fmt;
use std::fmt::Write as _;
use std::fs;
use std::num::NonZeroUsize;
use std::path::Path;
use std::path::PathBuf;
use std::slice;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::anyhow;
use anyhow::ensure;
use futures::future;
use nonempty::NonEmpty;
use textwrap::Options;
use tokio::time;
use tracing::instrument;

use crate::env::Env;
use crate::parser;
use crate::parser::Key;
use crate::parser::Line;
use crate::parser::LineKind;
use crate::svg::Frame;
use crate::svg::Theme;
use crate::tmux::Tmux;

/// Integration-test runner state.
pub struct Runner {
    /// Client for managing the `tmux` server. Ordered before `env` so that it is
    /// dropped first and cleans up the server before the environment is cleaned
    /// up, deleting its socket file.
    tmux: Tmux,

    /// Filesystem and process environment used for each test run.
    env: Env,

    /// One-based index of the next `:snap` directive, used to name SVG artifacts.
    snap_ix: usize,

    /// Path to the markdown transcript snapshot. SVG snapshots are written alongside it.
    snapshot_path: PathBuf,
}

impl Runner {
    /// Construct a runner with an isolated environment and tmux server.
    pub async fn new(
        manifest_dir: impl AsRef<Path>,
        snapshot_path: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let env = Env::new(manifest_dir.as_ref().to_path_buf()).await?;
        let tmux = Tmux::new(&env).await?;

        Ok(Self {
            tmux,
            env,
            snap_ix: 0,
            snapshot_path: snapshot_path.as_ref().to_path_buf(),
        })
    }

    /// Add a binary to the runner environment's `$PATH`.
    pub async fn bin(&self, bin: impl AsRef<OsStr>) -> anyhow::Result<()> {
        self.env.bin(bin).await?;
        Ok(())
    }

    /// Evaluate a parsed script and write markdown output for each line.
    pub async fn run(
        &mut self,
        w: &mut impl fmt::Write,
        script: &parser::Script<'_>,
    ) -> fmt::Result {
        let mut lines = script.lines.iter();
        while let Some(line) = lines.next() {
            let remaining = lines.clone();
            self.eval_line(w, line).await?;

            if let LineKind::Write { path } = &line.kind {
                self.eval_write(w, line.raw, path, remaining).await?;
            }
        }

        Ok(())
    }

    /// Gracefully shutdown the runner, waiting for all its components to exit.
    pub async fn shutdown(self) -> anyhow::Result<()> {
        self.tmux.shutdown().await?;
        Ok(())
    }

    /// Capture the active pane as styled cells and normalize dynamic spans with `filters`.
    async fn capture_frame(&self, filters: &[parser::Filter]) -> anyhow::Result<Frame> {
        let size = self
            .tmux
            .command("display-message")
            .args(["-p", "#{pane_height} #{pane_width}"])
            .status()
            .await?;

        let size = String::from_utf8_lossy(&size);
        let (rows, cols) = size
            .trim()
            .split_once(' ')
            .context("tmux did not report pane dimensions")?;

        let rows = rows.parse().context("invalid tmux pane height")?;
        let cols = cols.parse().context("invalid tmux pane width")?;

        let output = self
            .tmux
            .capture_pane()
            .await
            .context("failed to capture pane")?;

        Ok(Frame::parse(&output, rows, cols, filters))
    }

    /// Resolve requested binaries into the runner environment and report gaps.
    async fn eval_bins(&self, w: &mut impl fmt::Write, args: &[String]) -> fmt::Result {
        let futures = args.iter().map(|arg| self.env.bin(arg));
        let results = future::join_all(futures).await;

        let mut success = vec![];
        let mut failure = vec![];
        for (arg, result) in args.iter().zip(results) {
            match result {
                Ok(_) => success.push(arg.as_str()),
                Err(error) => failure.push((arg.as_str(), format!("{error:#}"))),
            }
        }

        let mut add_space = false;
        match &success[..] {
            [] => {}
            [bin] => {
                write_callout(w, "NOTE", &[&format!("'{bin}' is available.")])?;
                add_space = true;
            }

            [heads @ .., last] => {
                let mut line = String::new();

                let mut prefix = "";
                for bin in heads {
                    line.push_str(prefix);
                    write!(line, "'{bin}'")?;
                    prefix = ", ";
                }

                write!(line, ", and '{last}' are available.")?;
                write_callout(w, "NOTE", &[&line])?;
                add_space = true;
            }
        }

        for (bin, err) in &failure {
            if add_space {
                writeln!(w)?;
            }

            let line = format!("'{bin}' is unavailable: {err}");
            write_callout(w, "WARNING", &[&line])?;
            add_space = true;
        }

        Ok(())
    }

    /// Copy a fixture file into the sandboxed home directory.
    async fn eval_copy(
        &self,
        w: &mut impl fmt::Write,
        raw: &str,
        source: &Path,
        path: &Path,
    ) -> fmt::Result {
        write!(w, "{raw}")?;

        if let Err(error) = self.env.copy_file(source, path).await {
            writeln!(w, "\n")?;
            let msg = format!("failed to copy test file: {error:#}");
            write_callout(w, "WARNING", &[&msg])?;
        } else {
            writeln!(w, " (copied)")?;
        }

        Ok(())
    }

    /// Send parsed key presses to the active pane.
    async fn eval_keys(&self, w: &mut impl fmt::Write, raw: &str, keys: &[Key]) -> fmt::Result {
        writeln!(w, "{raw}")?;

        let command = self
            .tmux
            .command("send-keys")
            .args(keys.iter().map(|k| k.code().into_owned()));

        if let Err(e) = command.status().await {
            let stderr = e.to_string();
            let stderr = stderr.trim();
            let msg = if !stderr.is_empty() {
                format!("failed to send keys: {stderr}")
            } else {
                "failed to send keys".to_owned()
            };

            writeln!(w)?;
            write_callout(w, "WARNING", &[&msg])?;
        }

        Ok(())
    }

    /// Evaluate one parsed line and append its rendered markdown output.
    #[instrument(level = "trace", skip(self, w, line), fields(raw = line.raw))]
    async fn eval_line(&mut self, w: &mut impl fmt::Write, line: &Line<'_>) -> fmt::Result {
        match &line.kind {
            LineKind::Text => {
                writeln!(w, "{}", line.raw)?;
            }

            LineKind::Error { message } => {
                writeln!(w, "{}", line.raw)?;
                writeln!(w)?;
                write_callout(w, "WARNING", &[&format!("Parser error: {message}")])?;
            }

            LineKind::Bins { args } => {
                writeln!(w, "{}", line.raw)?;
                writeln!(w)?;
                self.eval_bins(w, args).await?;
            }

            LineKind::Sh { args } => {
                self.eval_sh(w, line.raw, args).await?;
            }

            // Handled in the main loop (this function's caller), so it can gather the file
            // contents from the following fenced code block.
            LineKind::Write { .. } => {}

            LineKind::Copy { source, path } => {
                self.eval_copy(w, line.raw, source, path).await?;
            }

            LineKind::Tmux { args } => {
                self.eval_tmux(w, line.raw, args).await?;
            }

            LineKind::Pane { target } => {
                writeln!(w, "{}", line.raw)?;
                self.eval_pane(w, target).await?;
            }

            LineKind::Keys { keys } => {
                self.eval_keys(w, line.raw, keys).await?;
            }

            LineKind::Settle {
                count,
                duration,
                filters,
            } => {
                self.eval_settle(w, line.raw, *count, *duration, filters)
                    .await?;
            }

            LineKind::Snap {
                count,
                duration,
                color,
                filters,
            } => {
                writeln!(w, "{}", line.raw)?;
                writeln!(w)?;
                self.eval_snap(w, *count, *duration, *color, filters)
                    .await?;
            }
        }

        Ok(())
    }

    /// Switch the runner to a different active pane when the target exists.
    async fn eval_pane(&mut self, w: &mut impl fmt::Write, target: &str) -> fmt::Result {
        let pane = match self.target_to_pane_id(target).await {
            Ok(pane) => pane,
            Err(e) => {
                writeln!(w)?;
                let message = format!("failed to validate pane target '{target}': {e}");
                write_callout(w, "WARNING", &[&message])?;
                return Ok(());
            }
        };

        let Some(pane) = pane else {
            writeln!(w)?;
            write_callout(w, "WARNING", &["No such pane."])?;
            return Ok(());
        };

        if let Err(error) = self
            .tmux
            .command("switch-client")
            .args(["-t", &pane])
            .status()
            .await
        {
            writeln!(w)?;
            let message = format!("failed to switch client to pane '{target}': {error}");
            write_callout(w, "WARNING", &[&message])?;
            return Ok(());
        }

        // tmux control-mode subscriptions are throttled, so confirm the switch with an explicit
        // query instead of waiting for the next subscription notification.
        match self.tmux.refresh_pane().await {
            Ok(current) => {
                if current != pane {
                    writeln!(w)?;
                    let msg = format!("failed to observe pane target '{target}'");
                    write_callout(w, "WARNING", &[&msg])?;
                }
            }
            Err(error) => {
                writeln!(w)?;
                let message = format!("failed to observe pane target '{target}': {error}");
                write_callout(w, "WARNING", &[&message])?;
            }
        }

        Ok(())
    }

    /// Wait for the pane to reach a settled state within `deadline`. Repeatedly takes snapshots
    /// until `count` consecutive snapshots match.
    async fn eval_settle(
        &self,
        w: &mut impl fmt::Write,
        raw: &str,
        count: NonZeroUsize,
        duration: Duration,
        filters: &[parser::Filter],
    ) -> fmt::Result {
        write!(w, "{raw}")?;
        match self.settle(count, duration, filters).await {
            Ok(_) => writeln!(w, " (settled)"),
            Err(e) => {
                writeln!(w, "\n")?;
                write_callout(w, "WARNING", &[&format!("{e:#}")])
            }
        }
    }

    /// Run a host command inside the runner environment and render its output.
    async fn eval_sh(
        &self,
        w: &mut impl fmt::Write,
        raw: &str,
        args: &NonEmpty<String>,
    ) -> fmt::Result {
        write!(w, "{raw}")?;

        let mut command = self.env.command(&args.head);

        // Indicate that this shell command is running in the context of the runner's tmux socket
        // and pane.
        let tmux = format!("{},,0", self.tmux.socket().display());
        command.env("TMUX", tmux).env("TMUX_PANE", self.tmux.pane());

        match command.args(&args.tail).output().await {
            Ok(output) => {
                if let Some(code) = output.status.code() {
                    writeln!(w, " (exit: {code})")?;
                } else {
                    writeln!(w, " (exit: killed)")?;
                }

                if !output.stdout.is_empty() {
                    writeln!(w)?;
                    write_fenced_block(w, "stdout", &String::from_utf8_lossy(&output.stdout))?;
                }

                if !output.stderr.is_empty() && !output.status.success() {
                    writeln!(w)?;
                    write_fenced_block(w, "stderr", &String::from_utf8_lossy(&output.stderr))?;
                }
            }

            Err(e) => {
                writeln!(w, "\n")?;
                let msg = format!("failed to execute command: {e}");
                write_callout(w, "WARNING", &[&msg])?;
            }
        }

        Ok(())
    }

    /// Capture the current pane in a settled state within `deadline`. Repeatedly takes snapshots
    /// until `count` consecutive snapshots match.
    async fn eval_snap(
        &mut self,
        w: &mut impl fmt::Write,
        count: NonZeroUsize,
        duration: Duration,
        color: bool,
        filters: &[parser::Filter],
    ) -> fmt::Result {
        self.snap_ix += 1;
        match self.settle(count, duration, filters).await {
            Ok(frame) => {
                write_fenced_block(w, "terminal", frame.text())?;
                if color {
                    writeln!(w)?;
                    write_svg(w, &self.snapshot_path, self.snap_ix, &frame, Theme::Light)?;
                    write_svg(w, &self.snapshot_path, self.snap_ix, &frame, Theme::Dark)?;
                }
                Ok(())
            }
            Err(e) => write_callout(w, "WARNING", &[&format!("{e:#}")]),
        }
    }

    /// Run a tmux command against the test server and render its outcome.
    async fn eval_tmux(
        &self,
        w: &mut impl fmt::Write,
        raw: &str,
        args: &NonEmpty<String>,
    ) -> fmt::Result {
        write!(w, "{raw}")?;

        let command = self.tmux.command(&args.head).args(&args.tail);
        match command.status().await {
            Ok(output) => {
                writeln!(w, " (success)")?;

                let output = String::from_utf8_lossy(&output);
                let output = output.trim();
                if !output.is_empty() {
                    writeln!(w)?;
                    write_fenced_block(w, "", output)?;
                }
            }

            Err(e) => {
                writeln!(w, " (failure)")?;

                let output = e.to_string();
                let output = output.trim();
                if !output.is_empty() {
                    writeln!(w)?;
                    write_fenced_block(w, "", output)?;
                }
            }
        }

        Ok(())
    }

    /// Write a fenced block into the sandboxed home directory.
    async fn eval_write(
        &self,
        w: &mut impl fmt::Write,
        raw: &str,
        path: &Path,
        mut lines: slice::Iter<'_, Line<'_>>,
    ) -> fmt::Result {
        write!(w, "{raw}")?;

        for line in &mut lines {
            if let LineKind::Text = line.kind {
                if line.raw.trim().is_empty() {
                    continue;
                }

                if line.raw.starts_with("```") {
                    break;
                }
            }

            let msg = "Parser error: ':write' expects to be followed by a fenced code block";
            writeln!(w, "\n")?;
            return write_callout(w, "WARNING", &[msg]);
        }

        let mut contents = String::new();
        loop {
            let Some(line) = lines.next() else {
                writeln!(w, "\n")?;
                let msg = "Parser error: unexpected end of input in ':write' block";
                return write_callout(w, "WARNING", &[msg]);
            };

            let LineKind::Text = line.kind else {
                writeln!(w, "\n")?;
                let msg = "Parser error: ':write' expects a fenced code block with text content";
                return write_callout(w, "WARNING", &[msg]);
            };

            if line.raw.starts_with("```") {
                break;
            }

            contents.push_str(line.raw);
            contents.push('\n');
        }

        if let Err(error) = self.env.write_file(path, &contents).await {
            writeln!(w, "\n")?;
            let msg = format!("failed to write file: {error:#}");
            write_callout(w, "WARNING", &[&msg])?;
            return Ok(());
        }

        writeln!(w, " (written)")?;
        Ok(())
    }

    /// Capture the current pane in a settled state within `duration`.
    ///
    /// A settled state implies that a streak of `count` snapshots all observed the same state,
    /// after filters have been applied.
    async fn settle(
        &self,
        count: NonZeroUsize,
        duration: Duration,
        filters: &[parser::Filter],
    ) -> anyhow::Result<Frame> {
        const INTERVAL: Duration = Duration::from_millis(25);

        let deadline = time::Instant::now() + duration;
        let mut capture = None;
        let mut streak = 0;
        let target = count.get();

        loop {
            let frame = self
                .capture_frame(filters)
                .await
                .context("failed to capture pane")?;
            let pane = frame.text().to_owned();

            match &mut capture {
                _ if pane.trim().is_empty() => {
                    // Ignore empty captures, they usually indicate that tmux hasn't initialized
                    // the pane yet.
                }

                Some(prev) if prev == &pane => {
                    streak += 1;
                }

                Some(prev) => {
                    *prev = pane;
                    streak = 1;
                }

                None => {
                    capture = Some(pane);
                    streak = 1;
                }
            }

            if streak >= target {
                return Ok(frame);
            }

            time::sleep(INTERVAL).await;
            ensure!(
                time::Instant::now() <= deadline,
                "pane did not stabilize in {}ms",
                duration.as_millis()
            );
        }
    }

    /// Resolve a user-facing pane target into a concrete tmux pane id.
    async fn target_to_pane_id(&self, target: &str) -> anyhow::Result<Option<String>> {
        let output = self
            .tmux
            .command("display-message")
            .args(["-p", "-t", target])
            .arg("#{pane_id}\t#{session_name}:#{window_index}.#{pane_index}")
            .status()
            .await?;

        let output = String::from_utf8_lossy(&output);
        let candidates: Vec<_> = output
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .collect();

        if let [(pane, _)] = &candidates[..]
            && pane.starts_with('%')
        {
            Ok(Some((*pane).to_owned()))
        } else {
            Ok(None)
        }
    }
}

/// Write a GitHub-style markdown callout, wrapping content to the repo line
/// width.
fn write_callout<S: AsRef<str>>(w: &mut impl fmt::Write, kind: &str, lines: &[S]) -> fmt::Result {
    writeln!(w, "> [!{kind}]")?;

    for line in lines {
        for line in line.as_ref().split('\n') {
            if line.is_empty() {
                writeln!(w, ">")?;
                continue;
            }

            let opts = Options::new(100).break_words(false);
            let wrapped = if let Some(rest) = line.strip_prefix("- ") {
                textwrap::wrap(rest, opts.initial_indent("> - ").subsequent_indent(">   "))
            } else {
                textwrap::wrap(line, opts.initial_indent("> ").subsequent_indent("> "))
            };

            for line in wrapped {
                writeln!(w, "{line}")?;
            }
        }
    }

    Ok(())
}

/// Write a fenced code block, ensuring the block always ends with a trailing
/// newline.
fn write_fenced_block(w: &mut impl fmt::Write, label: &str, text: &str) -> fmt::Result {
    writeln!(w, "```{label}")?;
    write!(w, "{text}")?;

    if !text.ends_with('\n') {
        writeln!(w)?;
    }

    writeln!(w, "```")?;
    Ok(())
}

/// Writes one SVG rendering for a pane snapshot, and embeds it in the transcript.
fn write_svg(
    w: &mut impl fmt::Write,
    snapshot_path: &Path,
    snap_ix: usize,
    frame: &Frame,
    theme: Theme,
) -> fmt::Result {
    let theme_name = match theme {
        Theme::Light => "light",
        Theme::Dark => "dark",
    };

    let mut path = snapshot_path.to_owned();
    path.add_extension(format!("{snap_ix}.{theme_name}.svg"));
    if let Err(e) = fs::write(&path, frame.render_svg(theme)) {
        writeln!(w)?;
        let msg = format!("{:#}", anyhow!(e).context("failed to write SVG snapshot"));
        write_callout(w, "WARNING", &[&msg])?;
    } else {
        let name = path.file_name().expect("SVG path must have a file name");
        writeln!(w, "![{}]({})", theme_name, name.display())?;
    }

    Ok(())
}
