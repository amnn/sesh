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

If you want to surface additional repositories, pass one or more repository
globs to `sesh`:

```tmux
bind s display-popup -E -w 80% -h 80% -T sesh -d "#{pane_current_path}" \
  "sesh -r ~/Code/'*' -r ~/.bootstrap -r ~/.config/nvim"
```

Reload tmux after editing your config:

```sh
tmux source-file ~/.tmux.conf
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

## Configuration

`sesh` reads configuration from `$XDG_CONFIG_HOME/sesh/sesh.toml`, or
`~/.config/sesh/sesh.toml` when `$XDG_CONFIG_HOME` is unset. You can also pass
an explicit config file with `--config PATH`.

The config file is optional. The default configuration does not run any extra
setup after creating a tmux session.

Use `[tmux].setup` to run a shell script after `sesh` creates a detached tmux
session. The script runs in the new session's tmux context and working
directory, so commands can use default tmux targets and relative paths:

```toml
[tmux]
setup = '''
tmux rename-window shell
tmux new-window -n editor 'nvim .'
'''
```
