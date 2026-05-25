# Adaptive layout

This scenario verifies that the picker redraws when the terminal width changes.
At narrow widths the session list and preview are stacked vertically; once the
terminal is wide enough they switch to a side-by-side layout.

    :bins jj cat

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init alpha
    :$ jj describe -R alpha -m "alpha commit"
    :t new-session -d -s ui "sesh -r alpha"
    :t resize-window -t ui:0 -x 120 -y 10
    :pane ui:0.0
    :settle

This snapshot shows the narrow stacked layout, including the horizontal
separator between the session list and preview.

    :k alpha
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

After widening the terminal, the picker should redraw with the session list and
preview side by side.

    :t resize-window -t ui:0 -x 180 -y 10
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

---
vim: set ft=markdown:
