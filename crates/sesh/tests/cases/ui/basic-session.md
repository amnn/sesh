# Basic session

This scenario creates several plain tmux sessions with no repo metadata and
then launches `sesh cli`.

:bins jj sleep cat

:t rename-session -t 0 runner
:t new-session -d -s alpha "cat"
:t new-session -d -s beta "cat"
:t new-session -d -s gamma "cat"
:t new-session -d -s ui "sesh cli"
:t resize-window -t ui:0 -x 80 -y 10
:$ sleep 1

:pane ui:0.0

This snapshot shows the initial picker state before any query is typed, so it
should list all discovered tmux sessions.

:snap

This snapshot shows the picker after typing `bet`, so the selection should move
to the `beta` session.

:k bet
:$ sleep 1
:snap

---
vim: set ft=markdown:
