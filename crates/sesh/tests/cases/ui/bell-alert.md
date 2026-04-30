# Bell alert

This scenario creates one quiet tmux session and one session with a bell alert,
then launches `sesh`. Pane capture does not expose styling, so these snapshots
are expected to look the same as non-alerted rows until the runner can assert
cell styles.

    :bins jj cat

    :t set-option -g visual-bell both
    :t set-window-option -g monitor-bell on
    :t rename-session -t 0 runner
    :t new-session -d -s alert "printf '\\a'; cat"
    :t new-session -d -s quiet "cat"
    :t new-session -d -s ui "sesh"
    :t resize-window -t ui:0 -x 80 -y 10
    :pane ui:0.0

The alert row should carry green pip styling, but the textual snapshot should
not show a difference.

    :settle
    :k alert
    :snap

The quiet row is not alerted and should look textually identical.

    :k C-u quiet
    :snap

---
vim: set ft=markdown:
