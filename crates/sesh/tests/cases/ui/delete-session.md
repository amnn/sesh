# Delete session

Plain live tmux sessions are closeable, but not deletable: there is no
associated checkout for `C-d` to remove.

    :bins jj tmux cat

    :t rename-session -t 0 runner
    :t new-session -d -s alpha "cat"
    :t new-session -d -s alpine "cat"
    :t new-session -d -s ui "sesh; cat"
    :t resize-window -t ui:0 -x 120 -y 10
    :pane ui:0.0
    :settle

Filter to the matching sessions. The first match, `alpha`, is selected. The
header should offer `C-x` close, but not `C-d` delete.

    :k alp
    :snap

Pressing `C-d` should do nothing for a plain live tmux session.

    :k C-d
    :settle
    :snap --color

The session should still exist, and the picker should still be running.

    :t has-session -t alpha
    :t has-session -t ui

---
vim: set ft=markdown:
