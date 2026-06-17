## Validation

Once you are happy with a solution, run the `nits` subagent to validate and
auto-fix style issues so the solution meets [code style guidelines](./STYLE.md).

Use a fix-first invocation: `@nits Fix style issues in-place; do not only
suggest changes. Report unresolved items with reasons.`

> [!IMPORTANT]
> Invoke it as a subagent (`@nits`), not as a shell command -- **there is no
> `nits` binary**.

When validating multiple Rust tests, avoid parallel `cargo test` invocations:
they contend on Cargo's package and build locks. Prefer a single `cargo
nextest run` command that covers the desired cases.

For Python maintenance scripts in `scripts/`, run `python3 -m py_compile` and a
small end-to-end fixture when the script mutates jj or Git metadata.

## Snapshots

For markdown-driven snapshot changes, refresh the checked-in `.snap` files with
`cargo insta test --accept` using the appropriate package/test selection, and
remove any leftover `.snap.new` artifacts before finishing.

Use `:snap --color` only when terminal colour is part of the behavior under
test; plain `:snap` intentionally skips SVG artifacts.

Do not add `:snap` replacement filters speculatively; use them only when the
captured output is unstable without them or when a test explicitly covers
replacement behavior.

When UI snapshots include `jj log --template builtin_log_compact` output, keep
volatile IDs and timestamps behind explicit `:snap` filters. For colour
snapshots, also keep `crates/sesh/tests/fixtures/jjconfig.toml` styling the
`change_id`/`commit_id` prefix and rest labels identically so jj's variable
unique-prefix boundary does not leak into SVG diffs.

When reviewing SVG snapshot diffs, inspect the actual SVG text/span changes and
compare old versus new before describing behavior. Distinguish visual movement
from changes in span ownership or styling of the same visible cells.

When a UI test sends keys immediately after starting or switching to a `sesh`
pane, use an explicit `:settle` directive before `:keys` to ensure the UI has
reached a stable state. For a freshly launched `sesh` pane, prefer
`:settle -d 2s`; the default timeout can be too short on cold runs.

When a UI test needs to assert behavior after `sesh` exits without
switching the client, keep the launched tmux pane alive (for example
`"sesh ...; cat"`) so later markdown directives can still query the tmux
server. Include any helper command used inside tmux panes (such as `cat` or
`sleep`) in the case's `:bins`; panes run with the sandboxed PATH, so missing
helpers can exit immediately and make sessions disappear.

## Architecture

Keep direct interactions with external binaries behind a dedicated module per
binary under `crates/sesh/src/cmd/`. For example, `tmux` command construction
and process execution belong in `crates/sesh/src/cmd/tmux.rs`, while `jj`
command construction and process execution belong in
`crates/sesh/src/cmd/jj.rs`. Other modules may decide when to request an
operation, but the binary-specific modules should abstract how that operation
is performed.

Keep `model` modules free of ratatui widgets and other concrete view types.
Session-specific rendering belongs in `app::sessions`, while generic reusable
widgets belong in `app::component`.

Only retain ratatui widgets for content that is both expensive and stable across
draw calls, such as loaded `jj log` text in preview or onto panes. Keep small or
highly dynamic chrome (prompt, header, separators) in immediate-mode style unless
profiling shows otherwise. When retaining a `Paragraph`, keep it in the owning
view and update its scroll before rendering it by reference; do not add a generic
retained-mode wrapper unless multiple views need the same abstraction.

When moving behavior onto domain types, keep configuration arguments narrow:
pass only the values the method needs rather than the full `SeshConfig`.

When adding or changing config fields, keep the schema, CLI long help, README
config examples, and markdown snapshot coverage in sync.

For read-only `jj` commands on startup or hot paths, pass
`--ignore-working-copy` unless fresh working-copy state is required; otherwise
large repositories can spend visible time snapshotting before `sesh` renders.

## Truth Seeking

Default to verified claims over plausible guesses.

- Do not assume facts about the codebase, runtime behavior, or external tools
  when they can be checked directly.
- Validate assumptions with repo evidence first (for example file reads,
  searches, tests, or command output) before acting on them.
- Do not justify or retain defensive changes from speculation. If a change is
  based on a plausible failure mode rather than observed evidence, either
  reproduce the failure, remove the change, or clearly call out the uncertainty
  before proceeding.
- Prefer to ground factual claims in external documentation or another citable
  source; make a reasonable effort to find one before relying on memory alone,
  and cite the source you used in the response.
- If a key fact cannot be verified safely, call out the uncertainty explicitly,
  state the recommended default, and explain what would change if that default
  is wrong.

## Change Stewardship

Preserve user intent and unrelated work when changing files.

- When asked to undo or narrowly adjust a change, preserve all unrelated text
  and operands exactly. Re-read the edited hunk or diff before reporting so the
  response does not claim a narrower change than was actually made.
- When the user manually edits code on top of agent changes, treat those edits
  as authoritative design feedback. Inspect the current diff and build on the
  user's version instead of reverting to an earlier agent approach.

## Licensing

Add the following comments to the top of every new source file:

```rust
// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0
```

## Reflection

After meaningful implementation work, use the `reflection` skill as the source
of truth for capturing durable lessons in repo-local agent guidance.

When updating repo guidance such as `AGENTS.md`, preserve the existing section
structure and formatting style; place new notes in the most specific section
and keep admonitions attached to the guidance they qualify.
