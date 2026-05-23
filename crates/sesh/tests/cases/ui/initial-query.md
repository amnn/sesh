# Initial query

The `--query` flag should seed the interactive filter before the picker draws.

    :bins jj cat tmux

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :t new-session -d -s beta "printf 'beta target'; cat"
    :t new-session -d -s gamma "printf 'gamma target'; cat"
    :t new-session -d -s ui "sesh --query gam"
    :t resize-window -t ui:0 -x 80 -y 10
    :pane ui:0.0
    :settle -d 2s

The prompt should contain the seeded query, and the list should be filtered to
matching sessions.

    :snap

---
vim: set ft=markdown:
