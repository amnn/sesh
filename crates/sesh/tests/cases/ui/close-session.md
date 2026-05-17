# Close session

`C-x` closes the selected live tmux session, then refreshes the session list
without closing the app or resetting the query.

    :bins jj tmux cat sleep

    :t rename-session -t 0 runner
    :t new-session -d -s alpha "cat"
    :t new-session -d -s alpine "cat"
    :t new-session -d -s ui "sesh; cat"
    :t resize-window -t ui:0 -x 120 -y 14
    :pane ui:0.0
    :settle

Filter to the matching sessions. The first match, `alpha`, is selected.

    :k alp
    :snap

Pressing `C-x` should kill `alpha`, keep `sesh` running, preserve the `alp`
query, and show the refreshed list with only `alpine` remaining.

    :k C-x
    :settle
    :snap

    :t has-session -t alpha

    :t has-session -t alpine
    :t has-session -t ui

---
vim: set ft=markdown:
