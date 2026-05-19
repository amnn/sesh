# Delete default workspace

Default jj workspace checkouts are not deletable from `sesh`; only named
workspaces can be deleted.

    :bins jj tmux cat test

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init beta
    :$ jj describe -R beta -m "beta commit"
    :t new-session -d -s ui "sesh -r beta; cat"
    :t resize-window -t ui:0 -x 120 -y 12
    :pane ui:0.0
    :settle -d 2s

Filter to the default workspace repo-only entry. The header should not offer
`C-d` delete.

    :k beta C-n
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

Pressing `C-d` should do nothing.

    :k C-d
    :settle
    :snap --color "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

The default checkout and picker session should both remain.

    :$ test -e beta
    :t has-session -t ui

---
vim: set ft=markdown:
