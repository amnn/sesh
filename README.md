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
