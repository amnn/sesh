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
