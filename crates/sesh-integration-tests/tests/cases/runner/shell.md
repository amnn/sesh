# Runner shell configuration

## Default pane prompt is stable

The default pane prompt should be stable across test environments.

    :bins env

    :tmux respawn-pane -k -t 0.0 'env ENV=$HOME/.shrc PS1="sh$ " /bin/sh -i'

    :tmux resize-window -x 80 -y 2 -t 0

    :snap

## New windows also have stable prompt

Creating a new window without a command should still produce the same stable prompt.

    :tmux new-window -d -n fresh 'env ENV=$HOME/.shrc PS1="sh$ " /bin/sh -i'
    :pane fresh.0
    :tmux resize-window -x 80 -y 2 -t fresh

    :snap

---
vim: set ft=markdown:
