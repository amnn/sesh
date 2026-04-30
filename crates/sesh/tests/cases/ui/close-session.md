# Close session

`C-x` closes the selected live tmux session. The shortcut is only offered for
live tmux sessions, not repo-only entries.

    :bins jj tmux cat sleep

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init beta
    :$ jj describe -R beta -m "beta commit"
    :t new-session -d -s alpha "cat"
    :t new-session -d -s ui "sesh -r beta; cat"
    :t resize-window -t ui:0 -x 120 -y 10
    :pane ui:0.0

The initial selection is the live `alpha` session, so the header offers close.

    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{6,8})\b/h"

Filtering to the repo-only `beta` row hides the close shortcut.

    :k beta
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{6,8})\b/h"

Pressing `C-x` on the live `alpha` session should kill `alpha` and exit without
switching to it.

    :k C-u C-x
    :$ sleep 1
    :t has-session -t alpha
    :t has-session -t ui

---
vim: set ft=markdown:
