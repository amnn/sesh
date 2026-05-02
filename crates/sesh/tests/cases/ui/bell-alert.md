# Bell alert

This scenario creates one quiet tmux session and one session with a bell alert,
then launches `sesh`. The markdown transcript remains textual, and the linked
SVG snapshots cover row styling.

    :bins jj cat

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t set-option -g visual-bell both
    :t set-window-option -g monitor-bell on
    :t rename-session -t 0 runner
    :t new-session -d -s alert "printf '\\a'; cat"
    :t new-session -d -s quiet "cat"
    :t new-session -d -s ui "sesh"
    :t resize-window -t ui:0 -x 80 -y 10
    :pane ui:0.0

The alert row should carry green pip styling in the linked SVG snapshot.

    :settle
    :k alert
    :snap --color

The quiet row is not alerted and should not carry the green pip styling.

    :k C-u quiet
    :snap --color

---
vim: set ft=markdown:
