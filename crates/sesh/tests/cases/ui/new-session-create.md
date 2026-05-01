# New session create

Selecting the ephemeral new-session row creates a session named by the query.
Without repo context, the new tmux session inherits the current working
directory and has no repo metadata.

    :bins jj tmux

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :t new-session -d -s ui "sesh"
    :t resize-window -t ui:0 -x 120 -y 10
    :pane ui:0.0
    :settle -d 2s

Type a unique session name and accept the new-session row.

    :k zeta
    :snap

    :k Enter
    :settle -d 2s

The client should switch to the newly-created session, and no repo metadata
should be attached.

    :t display-message -p '#{client_session}'

    :t list-sessions -F '#{session_name}:#{@sesh.repo}'

---
vim: set ft=markdown:
