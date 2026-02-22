//! Runtime for parsed markdown integration scripts.

use std::fmt;
use std::fmt::Write as _;
use std::path::Path;

use futures::future;
use futures::try_join;
use textwrap::Options;

use crate::env::Env;
use crate::parser;
use crate::parser::Line;
use crate::parser::LineKind;
use crate::sesh;
use crate::tmux::Tmux;

/// Integration-test runner state.
pub struct Runner {
    env: Env,
    tmux: Tmux,
}

impl Runner {
    /// Construct a runner with an isolated environment and tmux server under `tmp`.
    ///
    /// `tmp` should point at a test-owned temporary root (for example,
    /// `CARGO_TARGET_TMPDIR`) so all runner artifacts are scoped to the current test execution.
    pub async fn new(tmp: &Path) -> anyhow::Result<Self> {
        let env = Env::new(tmp).await?;

        let (_, tmux) = try_join!(
            async { env.bin(sesh::binary().await?).await },
            Tmux::new(&env)
        )?;

        Ok(Self { env, tmux })
    }

    pub async fn run(&self, w: &mut impl fmt::Write, script: &parser::Script<'_>) -> fmt::Result {
        for line in &script.lines {
            self.eval_line(w, line).await?;
        }

        Ok(())
    }

    async fn eval_line(&self, w: &mut impl fmt::Write, line: &Line<'_>) -> fmt::Result {
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
                let _ = target.len();
            }

            LineKind::Keys { keys } => {
                writeln!(w, "{}", line.raw)?;
                for key in keys {
                    let _ = (&key.kind, key.ctrl, key.meta, key.shft);
                }
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

    async fn eval_sh(&self, w: &mut impl fmt::Write, raw: &str, args: &[String]) -> fmt::Result {
        write!(w, "{raw}")?;

        let Some((program, tail)) = args.split_first() else {
            // This should be validated by the parser, but add a defensive check here.
            writeln!(w)?;
            write_callout(w, "WARNING", &["':sh' expects at least one argument"])?;
            return Ok(());
        };

        match self.env.command(program).args(tail).output().await {
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

    async fn eval_tmux(&self, w: &mut impl fmt::Write, raw: &str, args: &[String]) -> fmt::Result {
        write!(w, "{raw}")?;

        if args.is_empty() {
            // This should be validated by the parser, but add a defensive check here.
            writeln!(w)?;
            write_callout(w, "WARNING", &["':tmux' expects at least one argument"])?;
            return Ok(());
        }

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
}

fn write_callout(w: &mut impl fmt::Write, kind: &str, lines: &[&str]) -> fmt::Result {
    writeln!(w, "> [!{kind}]")?;

    for line in lines {
        for line in line.split('\n') {
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
