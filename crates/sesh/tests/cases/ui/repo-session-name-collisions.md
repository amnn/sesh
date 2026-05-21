# Repo session name collisions

When multiple discovered repos derive the same default session name, each repo
candidate should use the same suffix needed to avoid live tmux session names.
Repo candidates are distinguished by their paths rather than by increasing
suffixes.

    :bins jj tmux mkdir cat

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ mkdir -p alpha omega
    :$ jj git init alpha/beta
    :$ jj describe -R alpha/beta -m "alpha beta commit"
    :$ jj git init omega/beta
    :$ jj describe -R omega/beta -m "omega beta commit"
    :t new-session -d -s beta "cat"

Launch the picker with both repos discoverable. Both inactive repo rows should
show `beta~1`; the second repo should not be assigned `beta~2` just because
the first repo candidate already uses `beta~1`. Similarly the new-session row
will also show `beta~1`:

    :t new-session -d -s ui "sesh -r '*/beta'"
    :t resize-window -t ui:0 -x 120 -y 12
    :pane ui:0.0
    :settle -d 2s
    :k C-p beta
    :snap

---
vim: set ft=markdown:
