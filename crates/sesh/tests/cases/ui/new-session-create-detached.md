# New session create detached

Ctrl+n on the ephemeral new-session row creates the session without switching
the current tmux client or closing the picker.

    :bins jj tmux

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :t new-session -d -s ui "sesh"
    :t resize-window -t ui:0 -x 120 -y 10
    :pane ui:0.0
    :settle -d 2s

Type a unique session name and create it without switching.

    :k zeta
    :snap

    :k C-n
    :settle -d 2s

The picker should remain open with the query cleared, and the control-mode
client should still be attached to the picker session.

    :snap

    :t display-message -p '#{client_session}'

The new session should exist without repo metadata.

    :t list-sessions -F '#{session_name}:#{@sesh.repo}'

---
vim: set ft=markdown:
