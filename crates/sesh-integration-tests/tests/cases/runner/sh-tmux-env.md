# Shell tmux environment

Host shell commands should receive `TMUX` and `TMUX_PANE` values matching the
runner's current pane target, so tmux commands run through `:$` behave like
commands launched from that pane.

    :bins tmux cat

    :t rename-session -t 0 runner
    :t new-session -d -s beta "cat"
    :t list-clients -F '#{client_session}:#{pane_id}'

Switching from a shell directive should affect the control-mode client attached
to the runner's current pane.

    :$ tmux switch-client -t beta
    :t list-clients -F '#{client_session}:#{pane_id}'

    :t list-sessions -F '#{session_name}:#{session_attached}'

Changing the active runner pane via the synchronized pane directive should also
change the `TMUX_PANE` value seen by subsequent shell directives.

    :pane beta:0.0
    :$ tmux display-message -p '#{pane_id}'

---
vim: set ft=markdown:
