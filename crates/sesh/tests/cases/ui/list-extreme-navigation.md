# List extreme navigation

`M-up` and `M-down` jump to the first and last selectable rows in the session
list. The top spacer for an unavailable new session is not selectable, while a
valid new-session row is selectable.

    :bins jj cat

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :t new-session -d -s beta "cat"
    :t new-session -d -s delta "cat"
    :t new-session -d -s ta "cat"
    :t new-session -d -s zeta "cat"
    :t new-session -d -s ui "sesh"
    :t resize-window -t ui:0 -x 100 -y 12
    :pane ui:0.0
    :settle

Hide the preview so the session list has enough room to show several entries.
With no query, the top row is an unselectable spacer; `M-down` jumps to the
last row and `M-up` jumps back to the first selectable session.

    :k C-p
    :snap

    :k M-down
    :snap

    :k M-up
    :snap

A query for `et` keeps multiple existing matches and also allows a new session.
The new-session row is selectable, so `M-up` jumps to that row.

    :k et
    :snap

    :k M-down
    :snap

    :k M-up
    :snap

A query for `ta` also keeps multiple matches, but it exactly names the live
`ta` session. The new-session row is replaced by an unselectable spacer, so
`M-up` jumps to the first existing match instead.

    :k C-u ta
    :snap

    :k M-down
    :snap

    :k M-up
    :snap

---
vim: set ft=markdown:
