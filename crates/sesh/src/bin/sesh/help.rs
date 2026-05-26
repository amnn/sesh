// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Help output formatting for the `sesh` CLI.

use std::io;
use std::io::Write;

use anstyle::AnsiColor;
use anstyle::Style;
use clap::CommandFactory;
use clap::builder::Styles;
use textwrap::Options;
use unicode_width::UnicodeWidthStr as _;

/// Clap help colours, matching the picker UI highlight palette.
pub(crate) const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().bold())
    .usage(AnsiColor::Yellow.on_default().bold())
    .literal(AnsiColor::Blue.on_default().bold())
    .placeholder(AnsiColor::Blue.on_default());

/// Width reserved for keys in definition lists.
const KEY_WIDTH: usize = 16;

/// Writer that applies optional ANSI styles to custom help sections.
struct Writer<W> {
    inner: W,
    code: Style,
    color: bool,
    header: Style,
    literal: Style,
}

impl<W: Write + io::IsTerminal> Writer<W> {
    /// Create a writer, enabling ANSI styles only when the wrapped writer is a terminal.
    fn new(inner: W) -> Self {
        let color = inner.is_terminal();
        let (code, header, literal) = if color {
            (
                Style::new().dimmed(),
                AnsiColor::Yellow.on_default().bold(),
                AnsiColor::Blue.on_default().bold(),
            )
        } else {
            (Style::new(), Style::new(), Style::new())
        };

        Self {
            inner,
            code,
            color,
            header,
            literal,
        }
    }
}

impl<W: Write> Writer<W> {
    /// Run `inner` with code style applied.
    fn code(&mut self, inner: impl FnOnce(&mut Self) -> io::Result<()>) -> io::Result<()> {
        self.scope(self.code, inner)
    }

    /// Write a key-value (definition) pair. The key is styled as a literal, left aligned and
    /// occupying a fixed width, and the description follows, wrapped and indented after the key.
    fn def(&mut self, key: &str, desc: &str) -> io::Result<()> {
        let mut nest = self.nest();
        nest.literal(|w| write!(w, "  {key:<KEY_WIDTH$}  "))?;

        let initial_indent = nest.as_str()?;
        let indent = " ".repeat(KEY_WIDTH + 4);
        let opts = if key.width() > KEY_WIDTH {
            writeln!(self, "{initial_indent}")?;
            Options::with_termwidth()
                .initial_indent(&indent)
                .subsequent_indent(&indent)
        } else {
            Options::with_termwidth()
                .initial_indent(initial_indent)
                .subsequent_indent(&indent)
        };

        writeln!(self, "{}", textwrap::fill(desc, opts))
    }

    /// Run `inner` with header style applied.
    fn header(&mut self, inner: impl FnOnce(&mut Self) -> io::Result<()>) -> io::Result<()> {
        self.scope(self.header, inner)
    }

    /// Run `inner` with literal style applied.
    fn literal(&mut self, inner: impl FnOnce(&mut Self) -> io::Result<()>) -> io::Result<()> {
        self.scope(self.literal, inner)
    }

    /// Create a nested writer that shares the same styles but writes to an independent buffer.
    /// This is useful for formatting output in a separate buffer before writing it to the main
    /// output.
    fn nest(&self) -> Writer<Vec<u8>> {
        Writer {
            inner: Vec::new(),
            code: self.code,
            color: self.color,
            header: self.header,
            literal: self.literal,
        }
    }

    /// Write a paragraph of text -- indented uniformly and wrapped to the terminal width.
    fn paragraph(&mut self, text: &str) -> io::Result<()> {
        let options = Options::with_termwidth()
            .initial_indent("  ")
            .subsequent_indent("  ");

        write!(self.inner, "{}", textwrap::fill(text, options))
    }

    /// Apply `style` while `inner` writes to the wrapped stream.
    fn scope(
        &mut self,
        style: Style,
        inner: impl FnOnce(&mut Self) -> io::Result<()>,
    ) -> io::Result<()> {
        write!(self.inner, "{style}")?;
        inner(self)?;
        write!(self.inner, "{style:#}")
    }
}

impl Writer<Vec<u8>> {
    /// Return the nested buffer as UTF-8 text.
    fn as_str(&self) -> io::Result<&str> {
        str::from_utf8(&self.inner).map_err(io::Error::other)
    }
}

impl<W: Write> Write for Writer<W> {
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
}

/// Write the complete long help output.
///
/// This includes the clap-generated help for command-line arguments, a section on keybindings, and
/// a section on configuration.
pub(crate) fn write_long_help<C: CommandFactory>() -> io::Result<()> {
    let w = &mut Writer::new(io::stdout());

    let help = C::command().render_long_help();
    if w.color {
        write!(w, "{}", help.ansi())?;
    } else {
        write!(w, "{help}")?;
    }

    write_key_bindings(w)?;
    write_config_help(w)?;
    w.write_all(b"\n")
}

/// Write the complete custom configuration help section.
fn write_config_help<W: Write>(w: &mut Writer<W>) -> io::Result<()> {
    writeln!(w)?;
    w.header(|out| write!(out, "Configuration:"))?;

    writeln!(w)?;
    w.paragraph(
        "The config file is TOML. It is optional; when no file is present, sesh uses the \
         built-in defaults.",
    )?;

    writeln!(w)?;
    writeln!(w)?;

    w.def(
        "repo.globs",
        "Glob patterns for jj repositories to surface alongside existing tmux sessions. These \
         stack with any --repo command-line globs.",
    )?;

    w.def(
        "tmux.setup",
        "Shell script to run after sesh creates a detached tmux session. The script runs in the \
         new session's tmux context and working directory, so commands can use default tmux \
         targets and relative paths.",
    )?;

    writeln!(w)?;
    w.header(|out| writeln!(out, "Example:"))?;

    w.paragraph(
        "Below is an example configuration that makes use of every available configuration field.",
    )?;

    writeln!(w)?;
    writeln!(w)?;
    w.code(|out| {
        writeln!(out, "  [repo]")?;
        writeln!(out, "  globs = [")?;
        writeln!(out, "    \"/Users/alice/Code/*\",")?;
        writeln!(out, "    \"/Users/alice/.config/nvim\"")?;
        writeln!(out, "  ]")?;
        writeln!(out)?;
        writeln!(out, "  [tmux]")?;
        writeln!(out, "  setup = '''")?;
        writeln!(out, "  tmux rename-window shell")?;
        writeln!(out, "  tmux new-window -n editor 'nvim .'")?;
        writeln!(out, "  '''")
    })?;

    Ok(())
}

/// Write the complete custom key binding help section.
fn write_key_bindings<W: Write>(w: &mut Writer<W>) -> io::Result<()> {
    writeln!(w)?;
    w.header(|w| write!(w, "Key bindings:"))?;

    writeln!(w)?;
    w.def("C-d", "Delete the repository and close the session.")?;
    w.def("C-n", "Create the session without switching to it.")?;
    w.def("C-p", "Toggle the preview pane.")?;
    w.def("C-r, M-r", "Set or reset the current repo.")?;
    w.def("C-u", "Clear the filter.")?;
    w.def("C-x", "Close a live session.")?;
    w.def("C-y", "Confirm a pending deletion.")?;
    w.def("up, down", "Move selection by one row.")?;
    w.def("M-up, M-down", "Move selection to the first or last row.")?;
    w.def("S-up, S-down", "Scroll the preview pane up or down.")?;
    w.def("enter", "Switch to the session, creating it if necessary.")?;
    w.def("esc, C-g, C-c", "Close the UI.")?;

    Ok(())
}
