# Repo context shortcut

This scenario launches `sesh` from inside the `alpha` repository so the
picker starts with a repo context inferred from `cwd`, then uses `C-r` both on
an unfiltered repo row and on a row selected from a multi-match query.

    :bins jj cat

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init alpha
    :$ jj describe -R alpha -m "alpha commit"
    :$ jj git init beta
    :$ jj describe -R beta -m "beta commit"
    :$ jj git init gamma
    :$ jj describe -R gamma -m "gamma commit"
    :t new-session -d -s ui "cd alpha && sesh -r '../alpha' -r '../beta' -r '../gamma'"
    :t resize-window -t ui:0 -x 90 -y 10
    :pane ui:0.0

This snapshot shows the initial picker state with the current repo inferred
from `cwd`.

    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

Move the cursor to the discovered `beta` repo row.

    :k down down down
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

Hide the session preview while `beta` is selected. The list should use the
full height, and no repo log should be visible.

    :k C-p
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

Press `C-o` while the session preview is hidden. Onto mode should still reserve
and render the preview-shaped pane with the current repo context (`alpha`), not
the selected row's preview (`beta`).

    :k C-o
    :snap -d 2s "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

Cancel onto mode and restore the session preview before continuing with
repo-context shortcuts.

    :k C-g C-p

Press `C-r` to set repo context from the selected row. The header should update
to `beta` while the selected row stays on `beta` after the picker refreshes.

    :k C-r
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

This snapshot applies the query `a`, which still matches multiple repo rows,
moves to `gamma`, and then uses `C-r` to update the current repo from the
filtered list.

    :k C-u a down down C-r
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

Press `M-r` to clear the current repo without changing the selected row.

    :k M-r
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

---
vim: set ft=markdown:
