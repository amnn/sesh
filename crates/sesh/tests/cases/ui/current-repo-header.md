# Current repo header

This scenario launches `sesh cli` from inside a repository directory to verify
the picker header shows the repo inferred from the current working directory.

    :bins jj cat

    :t rename-session -t 0 runner
    :$ jj git init alpha
    :$ jj describe -R alpha -m "alpha commit"
    :t new-session -d -s alpha-live "cat"
    :t new-session -d -s ui "cd alpha && sesh cli"
    :t resize-window -t ui:0 -x 80 -y 10
    :pane ui:0.0

This snapshot shows the initial picker state, including the current repo in the
header.

    :snap -d 2s

---
vim: set ft=markdown:
