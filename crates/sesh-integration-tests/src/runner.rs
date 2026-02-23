//! Runtime for parsed markdown integration scripts.

use std::ffi::OsStr;
use std::fmt;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::ensure;
use futures::future;
use nonempty::NonEmpty;
use textwrap::Options;
use tracing::instrument;

use crate::env::Env;
use crate::parser;
use crate::parser::Key;
use crate::parser::Line;
use crate::parser::LineKind;
use crate::tmux::Tmux;

/// Integration-test runner state.
pub struct Runner {
    env: Env,
    pane: String,
    tmux: Tmux,
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
            env,
            pane: "".to_owned(),
            tmux,
        };

        struct Sink;
        impl fmt::Write for Sink {
            fn write_str(&mut self, _: &str) -> fmt::Result {
                Ok(())
            }
        }

        // Query the `tmux` server to set the initial pane target.
        runner.eval_pane(&mut Sink, "0.0").await?;
        ensure!(!runner.pane.is_empty(), "failed to query initial tmux pane");

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
                for filter in filters {
                    let _ = (filter.patt.as_str(), &filter.repl);
                }
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

        match self.tmux.command(&self.env).args(args).output().await {
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
                let msg = format!("failed to execute tmux command: {e}");
                write_callout(w, "WARNING", &[&msg])?;
            }
        }

        Ok(())
    }

    async fn eval_pane(&mut self, w: &mut impl fmt::Write, target: &str) -> fmt::Result {
        const TEMPLATE: &str = "#{pane_id}\t#{session_name}:#{window_index}.#{pane_index}";

        let output = self
            .tmux
            .command(&self.env)
            .args(["display-message", "-p", "-t", target, TEMPLATE])
            .output()
            .await;

        let output = match output {
            Ok(output) => output,
            Err(e) => {
                let message = format!("failed to validate pane target '{target}': {e}");
                write_callout(w, "WARNING", &[&message])?;
                return Ok(());
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let candidates: Vec<_> = stdout
            .lines()
            .filter_map(|line| {
                let (id, target) = line.split_once('\t')?;
                Some((id, target))
            })
            .collect();

        if output.status.success()
            && let [(pane, _)] = &candidates[..]
            && pane.starts_with('%')
        {
            self.pane = (*pane).to_owned();
        } else {
            write_callout(w, "WARNING", &["No such pane."])?;
        }

        Ok(())
    }

    async fn eval_keys(&self, w: &mut impl fmt::Write, raw: &str, keys: &[Key]) -> fmt::Result {
        writeln!(w, "{raw}")?;

        for key in keys {
            let code = key.code();

            let output = match self
                .tmux
                .command(&self.env)
                .args(["send-keys", "-t", &self.pane])
                .arg(code.as_ref())
                .output()
                .await
            {
                Ok(output) => output,
                Err(e) => {
                    let msg = format!("failed to send {} to pane '{}': {}", key, self.pane, e);
                    write_callout(w, "WARNING", &[&msg])?;
                    break;
                }
            };

            if output.status.success() {
                continue;
            }

            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr = stderr.trim();

            let msg = if !stderr.is_empty() {
                format!("failed to send {} to pane '{}': {}", key, self.pane, stderr)
            } else {
                format!("failed to send {} to pane '{}'", key, self.pane)
            };

            write_callout(w, "WARNING", &[&msg])?;
            break;
        }

        Ok(())
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
