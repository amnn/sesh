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

## Snapshots

For markdown-driven snapshot changes, refresh the checked-in `.snap` files with
`cargo insta test --accept` using the appropriate package/test selection, and
remove any leftover `.snap.new` artifacts before finishing.

Use `:snap --color` only when terminal colour is part of the behavior under
test; plain `:snap` intentionally skips SVG artifacts.

When a UI test sends keys immediately after starting or switching to a `sesh`
pane, use an explicit `:settle` directive before `:keys` to ensure the UI has
reached a stable state.

When a UI test needs to assert behavior after `sesh` exits without
switching the client, keep the launched tmux pane alive (for example
`"sesh ...; cat"`) so later markdown directives can still query the tmux
server.

## Architecture

Keep direct interactions with external binaries behind a dedicated module per
binary. For example, `tmux` command construction and process execution belong
in `crates/sesh/src/tmux.rs`, while `jj` command construction and process
execution belong in `crates/sesh/src/jj.rs`. Other modules may decide when to
request an operation, but the binary-specific modules should abstract how that
operation is performed.

When moving behavior onto domain types, keep configuration arguments narrow:
pass only the values the method needs rather than the full `SeshConfig`.

## Truth Seeking

Default to verified claims over plausible guesses.

- Do not assume facts about the codebase, runtime behavior, or external tools
  when they can be checked directly.
- Validate assumptions with repo evidence first (for example file reads,
  searches, tests, or command output) before acting on them.
- When asked to undo or narrowly adjust a change, preserve all unrelated text
  and operands exactly. Re-read the edited hunk or diff before reporting so the
  response does not claim a narrower change than was actually made.
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
- Keep questions targeted and minimal: only ask when the missing fact would
  materially change the implementation or create risk.

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
