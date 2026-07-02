# List navigation aliases

`C-j` and `C-k` move down and up by one row, while `M-j` and `M-k`
jump to the last and first selectable rows in the session list.

    :bins jj cat

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :t new-session -d -s alpha "cat"
    :t new-session -d -s beta "cat"
    :t new-session -d -s gamma "cat"
    :t new-session -d -s zeta "cat"
    :t new-session -d -s ui "sesh"
    :t resize-window -t ui:0 -x 100 -y 12
    :pane ui:0.0
    :settle -d 2s

Hide the preview so the selected row movement is easy to inspect.

    :k C-p
    :snap

`C-j` moves the selection down by one row.

    :k C-j
    :snap

`C-k` moves the selection back up by one row.

    :k C-k
    :snap

`M-j` jumps to the last selectable row.

    :k M-j
    :snap

`M-k` jumps back to the first selectable row.

    :k M-k
    :snap

---
vim: set ft=markdown:
