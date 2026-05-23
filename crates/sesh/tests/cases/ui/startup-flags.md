# Startup flags

The fzf-style startup flags should let scripts seed the initial query, skip the
UI when exactly one discovered session matches, and run a non-interactive fuzzy
filter.

    :bins jj cat tmux

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :t new-session -d -s beta "printf 'beta target'; cat"
    :t new-session -d -s gamma "printf 'gamma target'; cat"

`--select-1` should switch immediately when the initial query has exactly one
matching discovered session.

    :$ tmux respawn-pane -k 'sesh --query beta --select-1; cat'
    :settle -d 2s

    :t display-message -p '#{client_session}'

    :$ tmux switch-client -t runner
    :pane runner:0.0

`--exit-0` should exit without opening the UI when the initial query has no
matches, even though an interactive picker could offer a new-session row.

    :$ sesh --query zzz --exit-0

    :t display-message -p '#{client_session}'

`--filter` should avoid the UI, print matching sessions, and not switch on its
own.

    :$ sesh --query a --filter

    :t display-message -p '#{client_session}'

Adding `--select-1` to a filter should switch when exactly one discovered
session matches.

    :$ tmux respawn-pane -k 'sesh --query gamm --filter --select-1; cat'
    :settle -d 2s

    :t display-message -p '#{client_session}'

A non-interactive filter with no matches should report a failed selection.

    :$ sesh --query zzz --filter

---
vim: set ft=markdown:
