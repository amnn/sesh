# Flag session

Live tmux sessions can carry a persistent manual flag in `@sesh.flag`. A
flagged session should show an alert-like pip with distinct colour and offer an
`unflag` shortcut in the header.

    :bins jj tmux cat

    :t rename-session -t 0 runner
    :t new-session -d -s alpha "cat"
    :t new-session -d -s beta "cat"
    :t set-option -t beta @sesh.flag 1
    :t new-session -d -s ui "sesh; cat"
    :t resize-window -t ui:0 -x 100 -y 10
    :pane ui:0.0
    :settle -d 2s

Filter to the pre-flagged session. The header should offer `unflag`, and the
linked SVG snapshot should show the pip in the flag colour rather than the alert
colour.

    :k beta
    :snap --color

Pressing `C-f` should clear the persisted tmux option and update the header to
offer `flag` again.

    :k C-f
    :settle -d 2s
    :snap --color

    :t display-message -p -t beta '#{@sesh.flag}'

Pressing `C-f` again should set the persisted tmux option back to `1`.

    :k C-f
    :settle -d 2s
    :snap --color

    :t display-message -p -t beta '#{@sesh.flag}'

---
vim: set ft=markdown:
