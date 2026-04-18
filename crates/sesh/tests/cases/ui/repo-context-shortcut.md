# Repo context shortcut

This scenario launches `sesh cli` from inside the `alpha` repository so the
picker starts with a repo context inferred from `cwd`, then uses `C-r` both on
an unfiltered repo row and on a row selected from a multi-match query.

    :bins jj cat

    :t rename-session -t 0 runner
    :$ jj git init alpha
    :$ jj describe -R alpha -m "alpha commit"
    :$ jj git init beta
    :$ jj describe -R beta -m "beta commit"
    :$ jj git init gamma
    :$ jj describe -R gamma -m "gamma commit"
    :t new-session -d -s ui "cd alpha && sesh cli -r '../alpha' -r '../beta' -r '../gamma'"
    :t resize-window -t ui:0 -x 90 -y 10
    :pane ui:0.0

This snapshot shows the initial picker state with the current repo inferred
from `cwd`.

    :snap -d 2s "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{6,8})\b/h"

Move the cursor to the discovered `beta` repo row.

    :k down down down
    :snap -d 2s "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{6,8})\b/h"

Press `C-r` to set repo context from the selected row. The header should update
to `beta` while the selected row stays on `beta` after the picker refreshes.

    :k C-r
    :snap -d 2s "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{6,8})\b/h"

This snapshot applies the query `a`, which still matches multiple repo rows,
moves to `gamma`, and then uses `C-r` to update the current repo from the
filtered list.

    :k C-u a down down C-r
    :snap -d 2s "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{6,8})\b/h"

---
vim: set ft=markdown:
