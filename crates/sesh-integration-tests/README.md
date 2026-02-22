# sesh integration tests

These tests run `sesh` in a headless tmux server and snapshot pane output with
`tmux capture-pane`.

## Test case syntax

Test cases live in `tests/cases/*.md`.

- Lines starting with `:` are directives.
- Other markdown lines are copied verbatim into the snapshot transcript.

Supported directives:

- `:$` / `:sh <cmd...>`
  - Run a host command via Rust `Command`.
  - Arguments are parsed with `shlex`.
- `:t` / `:tmux <args...>`
  - Run a tmux command on the test socket.
- `:p` / `:pane <target>`
  - Set current pane target (default is `zz-sesh-ui-runner:0.0`).
- `:k` / `:keys <tokens...>`
  - Send key presses to the current pane.
  - Bare tokens are key names / combinations only.
  - Key names are lowercase only: `enter`, `up`, `down`, `left`, `right`,
    `backspace`, `shift`, `ctrl`, `opt`, `esc`, `tab`, `space`.
  - Modifiers are canonical uppercase only: `C-`, `M-`, `S-`.
  - Printable text must be quoted: `"abc"`.
- `:s` / `:snap [dregexdreplacementd ...]`
  - Capture current pane and append it in a fenced `terminal` code block.
  - Optional replacement rules are sed-style pairs, separated by whitespace.
  - Replacements are global and applied in order.

## Create a new test

```bash
cargo run -p sesh-integration-tests --bin new-test -- <name>
```

This creates `tests/cases/<name>.md` with a starter template and syntax
reference.

## Run tests

Run only this crate's integration test harness:

```bash
cargo test -p sesh-integration-tests --test test
```

Run the full workspace test suite:

```bash
cargo nextest run --workspace
```
