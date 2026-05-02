# Mixed repo lifecycle

This scenario combines several repo-backed options in one picker:

- an open tmux session already attached to the `alpha` repo,
- a discovered `beta` repo with no open session,
- and a discovered `mono` repo plus its `mono-ws` workspace.

The glob passed to `sesh` ensures repos without matching tmux sessions are
still listed.

The `:snap` filters use distinct replacement characters so the snapshot makes
it obvious what was normalized:

- `t` replaces timestamps.
- `w` replaces `jj` change IDs that follow the preview graph markers.
- `h` replaces short hexadecimal commit IDs.

    :bins jj git cat

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

Create an `alpha` repo that will be attached to a live tmux session.

    :t rename-session -t 0 runner
    :$ jj git init alpha
    :$ jj describe -R alpha -m "alpha commit"

Create a `beta` repo that is only discoverable via the glob passed to `sesh
cli`.

    :$ jj git init beta
    :$ jj describe -R beta -m "beta commit"

Create a `mono` repo plus a `mono-ws` workspace so the picker can show multiple
entries from the same underlying repo.

    :$ jj git init mono
    :$ jj describe -R mono -m "mono commit"
    :$ jj workspace add mono-ws -R mono
    :$ jj describe -R mono-ws -m "mono workspace commit"

Launch one attached session for `alpha` and then open the picker with globbed
repo discovery enabled for `alpha`, `beta`, and `mono*`.

    :t new-session -d -s alpha-live "cat"
    :t set-option -t alpha-live @sesh.repo alpha
    :t new-session -d -s ui "sesh -r 'alpha' -r 'beta' -r 'mono*'"
    :t resize-window -t ui:0 -x 120 -y 12
    :pane ui:0.0

This snapshot shows the initial mixed picker state before any query is typed,
including the attached `alpha-live` session and the glob-discovered repos.

    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

This snapshot shows the picker after typing `beta`, so the selection should
move away from the initial attached-session result and onto the discovered
`beta` repo.

    :k beta
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

This snapshot shows the picker after clearing the query with `C-u` and typing
`mono`, so the selection should switch to the discovered `mono` entries.

    :k C-u mono
    :snap --color "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

---
vim: set ft=markdown:
