# New session stale repo context

If the current repo context comes from stale tmux metadata and is not actually a
jj repo, accepting a new session should use that path as the tmux working
directory without attempting workspace creation or attaching repo metadata.

    :bins jj tmux mkdir cat sh sed

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ mkdir plain
    :t new-session -d -s plain "cat"
    :t set-option -t plain @sesh.repo plain
    :t new-session -d -s ui "sesh"
    :t resize-window -t ui:0 -x 120 -y 10
    :pane ui:0.0
    :settle -d 2s

Select the live session with stale repo metadata, set it as the current repo
context, then accept a new-session row.

    :k plain C-r C-u zeta
    :snap

    :k Enter
    :settle -d 2s

The new tmux session should start in `plain`, but it should not record stale
repo metadata because no jj workspace was created.

    :t display-message -p '#{client_session}'

    :$ sh -c 'tmux display-message -p -t zeta:0 "#{pane_current_path}" | sed "s#$PWD/##g"'

    :t list-sessions -F '#{session_name}:#{b:@sesh.repo}'

---
vim: set ft=markdown:
