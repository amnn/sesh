# Runner shell configuration

## Default pane prompt is stable

The default pane prompt should be stable across test environments.

:bins sleep

:$ sleep 0.2
:tmux resize-window -x 80 -y 2 -t 0

:snap

## New windows also have stable prompt

Creating a new window without a command should still produce the same stable prompt.

:tmux new-window -d -n fresh
:pane fresh.0
:tmux resize-window -x 80 -y 2 -t fresh
:$ sleep 0.2

:snap

---
vim: set ft=markdown:
