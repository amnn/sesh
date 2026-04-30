# sesh integration tests

These tests run markdown directives in a headless tmux server and snapshot pane output with
`tmux capture-pane`.

## Test case syntax

Test cases live in `tests/cases/**/*.md`.

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
  - Use this instead of `:tmux switch-client ...` when later `:keys`, `:sh`, or
    `:snap` directives should operate on the new pane; `:pane` waits for the
    control-mode pane notification to settle before the next directive runs.
- `:k` / `:keys <tokens...>`
  - Send key presses to the current pane.
  - Key names are lowercase only: `enter`, `up`, `down`, `left`, `right`,
    `backspace`, `btab`, `esc`, `tab`, `space`.
  - Modifiers are canonical uppercase only: `C-`, `M-`, `S-`.
  - `S-` only applies to arrow keys.
  - Anything that doesn't match the above is treated as a literal string to send.
- `:settle [-c <count>] [-d <duration>] [dregexdgrapheme ...]`
  - Wait for the current pane to settle without appending a snapshot.
  - Accepts the same options and filters as `:snap`.
- `:s` / `:snap [-c <count>] [-d <duration>] [dregexdgrapheme ...]`
  - Capture current pane and append it in a fenced `terminal` code block.
  - `-c` / `--count` sets the required consecutive matching captures and
    defaults to `5`.
  - `-d` / `--duration` sets the maximum settle time and defaults to `1s`.
  - Durations use human-readable values such as `100ms` or `5s`.
  - Optional replacement rules are `dregexdgrapheme`, separated by whitespace.
  - Replacements are global and applied in order, painting over matches with the replacement
     grapheme cluster.
  - If the regex has capture groups, only those groups' contents are painted.

## Run tests

Run only this crate's integration test harness:

```bash
cargo nextest run -p sesh-integration-tests --test test
```

Run the full workspace test suite:

```bash
cargo nextest run --workspace
```
