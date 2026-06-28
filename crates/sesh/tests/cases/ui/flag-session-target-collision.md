# Flag session target collision

Flagging a session should target the selected tmux session even when the current
session has a window whose name prefixes the selected session name.

    :bins jj tmux cat

    :t rename-session -t 0 runner
    :t new-session -d -s nvim "cat"
    :t new-session -d -s ui -n "nvim - ui" "sesh; cat"
    :t resize-window -t ui:0 -x 100 -y 10
    :pane ui:0.0
    :settle -d 2s

Select the `nvim` session from inside a `ui` window whose name also starts with
`nvim`. Pressing `C-f` should set `@sesh.flag` on `nvim`, not on `ui`.

    :k nvim C-f
    :settle -d 2s

    :t display-message -p -t nvim: '#{@sesh.flag}'

    :t display-message -p -t ui: '#{@sesh.flag}'

---
vim: set ft=markdown:
