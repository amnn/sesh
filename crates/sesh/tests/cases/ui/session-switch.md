# Session switch

This scenario launches `sesh cli` in the runner's attached tmux pane and verifies
that pressing Enter on a live session switches that client to the selected
session.

    :bins jj cat tmux

    :t rename-session -t 0 runner
    :t new-session -d -s beta "printf 'beta target'; cat"
    :t resize-window -t runner:0 -x 80 -y 10

Respawn the runner's current pane with `sesh cli`. Shell directives receive the
runner's `TMUX` and `TMUX_PANE` environment, so this targets the pane currently
shown by the control-mode client.

    :$ tmux respawn-pane -k 'sesh cli'

Type a query that selects `beta`, wait for the picker to redraw, and accept it.
The next snapshot should follow the control-mode client to the selected session
and capture the `beta` pane.

    :k bet
    :snap
    :k enter
    :snap -d 2s

---
vim: set ft=markdown:
