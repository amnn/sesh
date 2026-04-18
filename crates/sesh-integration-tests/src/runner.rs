// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Runtime for parsed markdown integration scripts.

use std::ffi::OsStr;
use std::fmt;
use std::fmt::Write as _;
use std::num::NonZeroUsize;
use std::path::Path;
use std::slice;
use std::time::Duration;

use anyhow::Context as _;
use futures::future;
use nonempty::NonEmpty;
use textwrap::Options;
use tokio::time;
use tracing::instrument;
use unicode_segmentation::UnicodeSegmentation;

use crate::env::Env;
use crate::parser;
use crate::parser::Key;
use crate::parser::Line;
use crate::parser::LineKind;
use crate::tmux::Tmux;

/// Integration-test runner state.
pub struct Runner {
    /// Client for managing the `tmux` server. Ordered before `env` so that it is
    /// dropped first and cleans up the server before the environment is cleaned
    /// up, deleting its socket file.
    tmux: Tmux,

    /// Filesystem and process environment used for each test run.
    env: Env,

    /// Active tmux pane identifier that receives subsequent commands.
    pane: String,
}

impl Runner {
    /// Construct a runner with an isolated environment and tmux server.
    pub async fn new(manifest_dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let env = Env::new(manifest_dir.as_ref().to_path_buf()).await?;
        let tmux = Tmux::new(&env).await?;

        let mut runner = Self {
            tmux,
            env,
            pane: "".to_owned(),
        };

        runner.pane = runner
            .target_to_pane_id("0.0")
            .await
            .context("failed to query initial tmux pane target '0.0'")?
            .context("initial tmux pane target '0.0' not found")?;

        Ok(runner)
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

    /// Capture the active pane and normalize dynamic spans with `filters`.
    async fn capture_pane(&self, filters: &[parser::Filter]) -> anyhow::Result<String> {
        let output = self
            .tmux
            .command("capture-pane")
            .args(["-p", "-t"])
            .arg(&self.pane)
            .status()
            .await?;

        let mut capture = String::from_utf8_lossy(&output).into_owned();
        for filter in filters {
            capture = paint_filter(&capture, filter);
        }

        Ok(capture)
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
            .arg("-t")
            .arg(&self.pane)
            .args(keys.iter().map(|k| k.code().into_owned()));

        if let Err(e) = command.status().await {
            let stderr = e.to_string();
            let stderr = stderr.trim();
            let msg = if !stderr.is_empty() {
                format!("failed to send keys to pane '{}': {}", self.pane, stderr)
            } else {
                format!("failed to send keys to pane '{}'", self.pane)
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

            LineKind::Snap {
                count,
                duration,
                filters,
            } => {
                writeln!(w, "{}", line.raw)?;
                writeln!(w)?;
                self.eval_snap(w, *count, *duration, filters).await?;
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

        if let Some(pane) = pane {
            self.pane = pane;
        } else {
            writeln!(w)?;
            write_callout(w, "WARNING", &["No such pane."])?;
        }

        Ok(())
    }

    /// Run a host command inside the runner environment and render its output.
    async fn eval_sh(
        &self,
        w: &mut impl fmt::Write,
        raw: &str,
        args: &NonEmpty<String>,
    ) -> fmt::Result {
        write!(w, "{raw}")?;

        match self.env.command(&args.head).args(&args.tail).output().await {
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
    /// until `count` consecutive snapshots match. Initially snapshots are taken at a longer
    /// interval, until a change is detected, at which point the interval is shortened.
    async fn eval_snap(
        &self,
        w: &mut impl fmt::Write,
        count: NonZeroUsize,
        duration: Duration,
        filters: &[parser::Filter],
    ) -> fmt::Result {
        const INTERVAL: Duration = Duration::from_millis(25);

        let deadline = time::Instant::now() + duration;
        let mut capture = None;
        let mut streak = 0;
        let target = count.get();

        loop {
            let pane = match self.capture_pane(filters).await {
                Ok(pane) => pane,
                Err(error) => {
                    let message = format!("failed to capture pane '{}': {error:#}", self.pane);
                    write_callout(w, "WARNING", &[&message])?;
                    return Ok(());
                }
            };

            match &mut capture {
                _ if pane.trim().is_empty() => {
                    // ignore empty captures, they usually indicate that tmux hasn't initialized
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
                write_fenced_block(w, "terminal", capture.as_deref().unwrap_or_default())?;
                return Ok(());
            }

            time::sleep(INTERVAL).await;
            if time::Instant::now() > deadline {
                break;
            }
        }

        let warning = format!("pane did not stabilize in {}ms", duration.as_millis());
        write_callout(w, "WARNING", &[&warning])?;
        Ok(())
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

/// Paint regex matches or capture groups with the configured replacement grapheme.
fn paint_filter(input: &str, filter: &parser::Filter) -> String {
    // Convert regex captures to a list of ranges. If there are no groups, the
    // entire match is treated as a single group.
    let mut ranges = vec![];
    for captures in filter.patt.captures_iter(input) {
        if captures.len() == 1 {
            let m = captures.get_match();
            ranges.push(m.start()..m.end());
        }

        for m in captures.iter().skip(1).flatten() {
            ranges.push(m.start()..m.end());
        }
    }

    // Sort ranges, and then merge overlapping ranges. All merged ranges will
    // share the same start as the predecessor they were merged into.
    ranges.sort_by_key(|r| (r.start, r.end));

    let mut i = 0;
    let mut j = 0;
    while i + j + 1 < ranges.len() {
        let (head, tail) = ranges.split_at_mut(i + 1);
        let a = &mut head[i];
        let b = &mut tail[j];

        if a.contains(&b.start) {
            a.end = a.end.max(b.end);
            b.start = a.start;
            j += 1;
        } else {
            i += j + 1;
        }
    }

    // Remove ranges that have been merged into a previous range.
    ranges.dedup_by_key(|r| r.start);

    fn paint(text: &str, grapheme: &str) -> String {
        grapheme.repeat(text.graphemes(true).count())
    }

    // Write output out, painting over matched captures.
    let mut last = 0;
    let mut output = String::with_capacity(input.len());
    for r in ranges {
        output.push_str(&input[last..r.start]);
        output.push_str(&paint(&input[r.clone()], &filter.paint));
        last = r.end;
    }

    output.push_str(&input[last..]);
    output
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
