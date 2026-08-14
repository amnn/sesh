# Scripting

`smth` accepts fzf-style startup flags for scripted bindings.

Repository context is selected independently from discovery:

- `-b`, `--base REPO` uses a repository or workspace as the base context. A
  named workspace uses its repository's default checkout when available,
  following the same resolution as current-directory inference.
- `-B`, `--no-base` suppresses repository inference from the current working
  directory.
- With neither option, the nearest repository containing the current working
  directory is used when available.
- `-o`, `--onto REV` sets the revision used as the base of newly created
  workspaces. It defaults to `trunk()` and requires a repository base when
  supplied explicitly.
- `-r`, `--repo GLOB` adds repositories to discovery; it does not select the
  base context.

The picker and filtering modes support these startup options:

- `-q`, `--query STR` seeds the interactive query.
- `-1`, `--select-1` switches immediately when the initial query has one
  match.
- `-0`, `--exit-0` exits instead of opening the UI when the initial query has
  no matches.
- `-f`, `--filter` skips the UI and prints matches for the query from `--query`;
  combine it with `-1` to switch when there is exactly one match.
- `--json` skips the UI and emits structured records for live sessions and
  repository candidates. It cannot be combined with `--filter` or `--select-1`.
  `--query` narrows the output with the picker's fuzzy matcher. `--exit-0` is
  ignored so that an empty result is always emitted as `[]`.

## Structured inspection

`smth --json` emits a JSON array. Every record includes its resolved tmux name
and live and deletion state. Repo-backed records include the normalized default
workspace path to pass to `--base`; plain sessions omit `base`. The `name` field
contains a named-workspace or plain-session operand and is omitted for a default
checkout. `path` is omitted when no checkout exists, `flagged` appears only for
live sessions, and `attention` and `agents` are omitted when empty.

Use the `base` and `name` fields together when constructing a lifecycle
command. Do not substitute a discovery glob or derive a target from the tmux
name: collision suffixes and repository metadata are resolved independently.
