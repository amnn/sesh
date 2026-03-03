//! Runtime for parsed markdown integration scripts.

use std::ffi::OsStr;
use std::fmt;
use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use std::collections::HashMap;

use anyhow::Context as _;
use futures::future;
use nonempty::NonEmpty;
use textwrap::Options;
use tokio::time;
use tokio::time::MissedTickBehavior;
use tracing::instrument;

use crate::env::Env;
use crate::parser;
use crate::parser::Key;
use crate::parser::Line;
use crate::parser::LineKind;
use crate::tmux::Tmux;

/// Integration-test runner state.
pub struct Runner {
    /// Client for managing the `tmux` server. Ordered before `env` so that it is dropped first and
    /// clean up the server before the environment is cleaned up, deleting its socket file.
    tmux: Tmux,
    env: Env,
    pane: String,
}

impl Runner {
    /// Construct a runner with an isolated environment and tmux server under `tmp`.
    ///
    /// `tmp` should point at a test-owned temporary root (for example,
    /// `CARGO_TARGET_TMPDIR`) so all runner artifacts are scoped to the current test execution.
    pub async fn new(tmp: &Path) -> anyhow::Result<Self> {
        let env = Env::new(tmp).await?;
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

    pub async fn run(
        &mut self,
        w: &mut impl fmt::Write,
        script: &parser::Script<'_>,
    ) -> fmt::Result {
        for line in &script.lines {
            self.eval_line(w, line).await?;
        }

        Ok(())
    }

    /// Gracefully shutdown the runner, waiting for all its components to exit.
    pub async fn shutdown(self) -> anyhow::Result<()> {
        self.tmux.shutdown().await?;
        Ok(())
    }

    #[instrument(level = "trace", skip(self, w, line), fields(raw = line.raw))]
    async fn eval_line(&mut self, w: &mut impl fmt::Write, line: &Line<'_>) -> fmt::Result {
        match &line.kind {
            LineKind::Text => {
                writeln!(w, "{}", line.raw)?;
            }

            LineKind::Error { message } => {
                writeln!(w, "{}", line.raw)?;
                write_callout(w, "WARNING", &[&format!("Parser error: {message}")])?;
            }

            LineKind::Bins { args } => {
                writeln!(w, "{}", line.raw)?;
                self.eval_bins(w, args).await?;
            }

            LineKind::Sh { args } => {
                self.eval_sh(w, line.raw, args).await?;
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

            LineKind::Snap { filters } => {
                writeln!(w, "{}", line.raw)?;
                self.eval_snap(w, filters).await?;
            }
        }

        Ok(())
    }

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
                    write_fenced_block(w, "stdout", &String::from_utf8_lossy(&output.stdout))?;
                }

                if !output.stderr.is_empty() && !output.status.success() {
                    write_fenced_block(w, "stderr", &String::from_utf8_lossy(&output.stderr))?;
                }
            }

            Err(e) => {
                writeln!(w)?;
                let msg = format!("failed to execute command: {e}");
                write_callout(w, "WARNING", &[&msg])?;
            }
        }

        Ok(())
    }

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
                    write_fenced_block(w, "", output)?;
                }
            }

            Err(e) => {
                writeln!(w, " (failure)")?;

                let output = e.to_string();
                let output = output.trim();
                if !output.is_empty() {
                    write_fenced_block(w, "", output)?;
                }
            }
        }

        Ok(())
    }

    async fn eval_pane(&mut self, w: &mut impl fmt::Write, target: &str) -> fmt::Result {
        let pane = match self.target_to_pane_id(target).await {
            Ok(pane) => pane,
            Err(e) => {
                let message = format!("failed to validate pane target '{target}': {e}");
                write_callout(w, "WARNING", &[&message])?;
                return Ok(());
            }
        };

        if let Some(pane) = pane {
            self.pane = pane;
        } else {
            write_callout(w, "WARNING", &["No such pane."])?;
        }

        Ok(())
    }

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
            write_callout(w, "WARNING", &[&msg])?;
        }

        Ok(())
    }

    /// Capture the current pane repeatedly over a short duration, applying `filters` to each
    /// capture to normalize dynamic content. If more than `SNAP_MIN_DOMINANCE` of the captures
    /// stabilize to the same content, write that content as a fenced block, otherwise write a
    /// warning callout.
    async fn eval_snap(&self, w: &mut impl fmt::Write, filters: &[parser::Filter]) -> fmt::Result {
        const SNAP_DURATION: Duration = Duration::from_millis(100);
        const SNAP_INTERVAL: Duration = Duration::from_millis(10);
        const SNAP_MIN_DOMINANCE: f64 = 0.75;

        let mut samples = HashMap::new();
        let mut total = 0usize;

        let mut ticker = time::interval(SNAP_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let start = time::Instant::now();
        loop {
            ticker.tick().await;
            if start.elapsed() >= SNAP_DURATION {
                break;
            }

            let capture = match self.capture_pane(filters).await {
                Ok(capture) => capture,
                Err(error) => {
                    let message = format!("failed to capture pane '{}': {error:#}", self.pane);
                    write_callout(w, "WARNING", &[&message])?;
                    return Ok(());
                }
            };

            *samples.entry(capture).or_insert(0usize) += 1;
            total += 1;
        }

        let Some((capture, count)) = samples.into_iter().max_by(|(_, l), (_, r)| l.cmp(&r)) else {
            write_callout(w, "WARNING", &["did not capture any pane samples"])?;
            return Ok(());
        };

        let ratio = count as f64 / total as f64;
        if ratio < SNAP_MIN_DOMINANCE {
            let warning = format!("pane did not stabilize in {}ms", SNAP_DURATION.as_millis());
            write_callout(w, "WARNING", &[&warning])?;
            return Ok(());
        }

        write_fenced_block(w, "terminal", &capture)?;
        Ok(())
    }

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
            capture = filter
                .patt
                .replace_all(&capture, filter.repl.as_str())
                .into_owned();
        }

        Ok(capture)
    }

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

fn write_fenced_block(w: &mut impl fmt::Write, label: &str, text: &str) -> fmt::Result {
    writeln!(w, "```{label}")?;
    write!(w, "{text}")?;

    if !text.ends_with('\n') {
        writeln!(w)?;
    }

    writeln!(w, "```")?;
    Ok(())
}
