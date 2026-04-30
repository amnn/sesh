# Session switch

This scenario launches `sesh` in the runner's attached tmux pane and verifies
that pressing Enter on a live session switches that client to the selected
session.

    :bins jj cat tmux

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :t new-session -d -s beta "printf 'beta target'; cat"
    :t resize-window -t runner:0 -x 80 -y 10

Respawn the runner's current pane with `sesh`. Shell directives receive the
runner's `TMUX` and `TMUX_PANE` environment, so this targets the pane currently
shown by the control-mode client.

    :$ tmux respawn-pane -k 'sesh'
    :settle -d 2s

Type a query that selects `beta`, wait for the picker to redraw, and accept it.
After accepting the selection, the control-mode client should switch to the selected session.

    :k bet
    :snap

    :k enter
    :settle -d 2s

    :t display-message -p '#{client_session}'

---
vim: set ft=markdown:
