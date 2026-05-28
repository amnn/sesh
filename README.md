# sesh: tmux session switcher

[![CI](https://github.com/amnn/sesh/actions/workflows/ci.yml/badge.svg)](https://github.com/amnn/sesh/actions/workflows/ci.yml)

A tmux-native session switcher, for navigating between and opening new sessions
based on jujutsu (jj) repositories and workspaces.

## Installation

`sesh` expects `tmux` and `jj` to be available on `$PATH`.

Install the latest version from this repository with Cargo:

```sh
cargo install --locked --git https://github.com/amnn/sesh --package sesh
```

Make sure Cargo's binary directory is on your `$PATH` so tmux can find the
installed `sesh` binary.

## Setup

Bind `sesh` to tmux's session-switcher key, and move tmux's built-in session
chooser to `C-b S`:

```tmux
bind s display-popup -E -w 80% -h 80% -T sesh -d "#{pane_current_path}" "sesh"
bind S choose-tree -s
```

The `-d "#{pane_current_path}"` option runs `sesh` from the pane that was active
when you pressed `C-b s`, so the picker can pre-populate its current repository
context from that pane's working directory.

If you want to surface additional repositories, add repository globs to your
config:

```toml
[repo]
globs = [
  "/Users/alice/Code/*",
  "/Users/alice/.bootstrap",
  "/Users/alice/.config/nvim"
]
```

You can also pass one or more repository globs to `sesh`. Command-line globs
stack with `repo.globs` from config:

```tmux
bind s display-popup -E -w 80% -h 80% -T sesh -d "#{pane_current_path}" \
  "sesh -r ~/Code/'*' -r ~/.bootstrap -r ~/.config/nvim"
```

Reload tmux after editing your config:

```sh
tmux source-file ~/.tmux.conf
```

## Scripting

`sesh` accepts fzf-style startup flags for scripted bindings:

- `-q`, `--query STR` seeds the interactive query.
- `-1`, `--select-1` switches immediately when the initial query has one
  match.
- `-0`, `--exit-0` exits instead of opening the UI when the initial query
  has no matches.
- `-f`, `--filter` skips the UI and prints matches for the query from
  `--query`; combine it with `-1` to switch when there is exactly one match.

## Key bindings

`sesh -h` prints brief CLI help. `sesh --help` prints complete help, including
all picker key bindings:

| Key | Action |
| --- | --- |
| `C-d` | Delete the repository and close the session. |
| `C-f` | Flag or unflag a live session. |
| `C-n` | Create the session if necessary without switching to it. |
| `C-p` | Toggle the preview pane. |
| `C-r`, `M-r` | Set or reset the current repo. |
| `C-u` | Clear the filter. |
| `C-x` | Close a live session. |
| `C-y` | Confirm a pending deletion. |
| `up`, `down` | Move selection by one row. |
| `M-up`, `M-down` | Move selection to the first or last row. |
| `S-up`, `S-down` | Scroll the preview pane up or down. |
| `enter` | Switch to the session, creating it if necessary. |
| `esc`, `C-g`, `C-c` | Close the UI. |

## Configuration

`sesh` reads configuration from `$XDG_CONFIG_HOME/sesh/sesh.toml`, or
`~/.config/sesh/sesh.toml` when `$XDG_CONFIG_HOME` is unset. You can also pass
an explicit config file with `--config PATH`.

The config file is optional. The default configuration has no repository globs
and does not run any extra setup after creating a tmux session.

Use `[repo].globs` to surface jj repositories alongside existing tmux sessions.
These stack with any `--repo`/`-r` globs supplied on the command line.

Use `[tmux].setup` to run a shell script after `sesh` creates a detached tmux
session. The script runs in the new session's tmux context and working
directory, so commands can use default tmux targets and relative paths:

```toml
[repo]
globs = [
  "/Users/alice/Code/*",
  "/Users/alice/.config/nvim"
]

[tmux]
setup = '''
tmux rename-window shell
tmux new-window -n editor 'nvim .'
'''
```

## Troubleshooting

### `sesh` does not detect the repository from the current directory

`sesh` detects the default repository context from the directory it starts in.
If the picker header does not show the expected `repo: ...` value, check the
path tmux uses when launching `sesh`:

```tmux
bind s display-popup -E -w 80% -h 80% -T sesh -d "#{pane_current_path}" "sesh"
```

The `-d "#{pane_current_path}"` option should be present so tmux starts `sesh`
from the pane that was active when you opened the picker.

To confirm the directory is inside a jj workspace, run the same jj check from
that pane:

```sh
cd /path/to/repo/or/subdirectory
jj workspace root
```

If `jj workspace root` fails, switch to a directory inside the jj checkout or
fix the tmux binding so `sesh` starts from the active pane's path.

### `sesh` does not detect the repository for an existing session

Live tmux sessions only have repository metadata when they were opened by
`sesh`, or when you set the `@sesh.repo` user option yourself. Plain tmux
sessions still appear in the picker, but `sesh` cannot associate them with a jj
repository.

Check the session's repo metadata with:

```sh
tmux show-options -t SESSION -qv @sesh.repo
```

The command should print the repository path for repo-backed sessions. Empty
output means `sesh` will treat the tmux session as a plain live session.

To fix this, create or open the session through `sesh`. If you know the correct
repository path and want to attach it manually, set the user option yourself:

```sh
tmux set-option -t SESSION @sesh.repo /path/to/repo
```

### Session flags do not appear as expected

`sesh` stores manual flags in the `@sesh.flag` tmux user option for each live
session. You can inspect or repair the flag outside the picker with:

```sh
tmux show-options -t SESSION -qv @sesh.flag
tmux set-option -t SESSION @sesh.flag 1
tmux set-option -t SESSION @sesh.flag ""
```

### Git tools do not detect a secondary jj workspace

Released versions of jj can create additional workspaces for a colocated Git
repository, but those workspaces do not automatically become Git worktrees. Git
commands and tools that require repository discovery may fail with `fatal: not a
git repository`, or may behave unsafely if the workspace has only a direct
`.git` pointer to the default workspace's `.git` directory.

Check the current workspace with:

```sh
cd /path/to/jj/workspace
/path/to/sesh/scripts/jj-workspace-colocate.py doctor
```

You can also inspect the Git metadata directly:

```sh
git rev-parse --show-toplevel
git rev-parse --path-format=absolute --git-dir
git rev-parse --path-format=absolute --git-common-dir
jj log --ignore-working-copy -r @- --no-graph -T 'commit_id ++ "\n"'
```

A healthy secondary workspace should have `--show-toplevel` equal to the jj
workspace root, `--git-dir` different from `--git-common-dir` (meaning Git is
using a linked worktree with its own HEAD and index), and `git rev-parse HEAD`
matching jj `@-` printed by the `jj log` command above.

The repository includes
[`scripts/jj-workspace-colocate.py`](scripts/jj-workspace-colocate.py) to
attach linked Git worktree metadata to an existing jj workspace and to keep that
metadata aligned later.

> [!WARNING]
> This is a workaround for jj releases that do not yet support colocated
> secondary workspaces. It writes Git worktree metadata under the default
> workspace's `.git/worktrees/` directory and writes `.jj/.gitignore` in the
> workspace. Inspect the script first and use it only when you are comfortable
> repairing Git worktree metadata directly.

To create missing Git worktree metadata or refresh existing metadata, run:

```sh
cd /path/to/jj/workspace
/path/to/sesh/scripts/jj-workspace-colocate.py sync
```

If you previously used the unsafe one-line hack where `.git` points directly at
the default workspace's `.git` directory, replace that file with a real Git
worktree pointer:

```sh
cd /path/to/jj/workspace
/path/to/sesh/scripts/jj-workspace-colocate.py sync --replace-existing
```

`sync` creates linked Git worktree metadata when it is missing. It also sets
Git's HEAD and index to jj `@-`, so workspace file changes appear to Git as
ordinary worktree modifications. If you later run jj commands that change `@-`,
such as `jj new`, `jj edit`, or a rebase from another workspace, run `sync`
again.

`sync` uses `git reset --mixed`, so it will unstage any changes that Git tools
staged. It does not update the working tree files.

### `sesh` does not associate a workspace with its default checkout

If `sesh` can find a jj checkout but new workspace names or paths are based on
the current workspace instead of the default checkout, the jj workspace path
index may be stale or missing. Check whether jj can report workspace paths:

```sh
cd /path/to/repo/or/subdirectory
jj workspace root
jj workspace list --template 'name ++ "\t" ++ root ++ "\n"'
```

If `jj workspace root` points at the right checkout but `jj workspace list`
prints `<Error: ... workspace_store/index ...>` or
`<Error: Workspace has no recorded path: ...>`, the repository was likely
created before jj recorded workspace paths. `sesh` can still find the `.jj`
directory, but it cannot tell which checkout is the default workspace.

The repository includes [`scripts/fix-jj-workspace-index.py`](scripts/fix-jj-workspace-index.py) to
recreate the missing workspace path index. It has been tested with `jj
0.40.0`.

> [!WARNING]
> This script writes jj repository metadata. Use it at your own risk, inspect
> the script first, and only run it when you are comfortable repairing the
> `.jj/repo/workspace_store/index` file directly. The script backs up the
> existing index file next to the original before replacing it.

Run it from the default workspace root with one `name=/path/to/checkout`
argument for each workspace. Include the default workspace too:

```sh
cd /path/to/default/repo
/path/to/sesh/scripts/fix-jj-workspace-index.py \
  default=/path/to/default/repo \
  feature=/path/to/feature/repo
jj workspace list --template 'name ++ "\t" ++ root ++ "\n"'
```

The final command should print each workspace with its checkout path instead of
an error. After that, jj will maintain the index when you add or forget
workspaces.
