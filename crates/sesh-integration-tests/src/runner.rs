//! Runtime for parsed markdown integration scripts.

use futures::future;
use futures::try_join;
use std::fmt::Write;
use std::path::Path;
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

    pub async fn run(&self, w: &mut impl Write, script: &parser::Script<'_>) -> anyhow::Result<()> {
        for line in &script.lines {
            self.eval_line(w, line).await?;
        }

        Ok(())
    }

    async fn eval_line(&self, w: &mut impl Write, line: &Line<'_>) -> anyhow::Result<()> {
        writeln!(w, "{}", line.raw)?;

        match &line.kind {
            LineKind::Text => {}

            LineKind::Error { message } => {
                write_callout(w, "WARNING", &[&format!("Parser error: {message}")])?;
            }

            LineKind::Bins { args } => {
                self.eval_bins(w, args).await?;
            }

            LineKind::Sh { args } => {
                let _ = args.len();
            }

            LineKind::Tmux { args } => {
                let _ = args.len();
                let _ = &self.tmux;
            }

            LineKind::Pane { target } => {
                let _ = target.len();
            }

            LineKind::Keys { keys } => {
                for key in keys {
                    let _ = (&key.kind, key.ctrl, key.meta, key.shft);
                }
            }

            LineKind::Snap { filters } => {
                for filter in filters {
                    let _ = (filter.patt.as_str(), &filter.repl);
                }
            }
        }

        Ok(())
    }

    async fn eval_bins(&self, w: &mut impl Write, args: &[String]) -> anyhow::Result<()> {
        let futures = args.iter().map(|arg| self.env.bin(arg));
        let results = future::join_all(futures).await;

        let mut success = vec![];
        let mut failure = vec![];
        for (arg, result) in args.iter().zip(results) {
            match result {
                Ok(_) => success.push(arg.as_str()),
                Err(error) => failure.push((arg.as_str(), error.to_string())),
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

                write!(line, " and '{last}' are available.")?;
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
}

fn write_callout(w: &mut impl Write, kind: &str, lines: &[&str]) -> anyhow::Result<()> {
    writeln!(w, "> [!{kind}]")?;

    for line in lines {
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

    Ok(())
}
