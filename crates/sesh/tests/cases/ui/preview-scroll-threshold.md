# Preview scroll threshold

This scenario verifies the preview scrollbar threshold. In this fixture, `jj`
still renders the graph markers, so the viewport-fitting case uses 11 content
lines plus the trailing root marker line:

- when the preview height exactly matches the viewport, no scrollbar is shown,
- when the preview is one line longer, a scrollbar appears,
- and once it appears, the preview can scroll.

The test configures `jj log` to show only commit descriptions so the preview is
easy to read in snapshots.

    :bins jj cat sh

    :t rename-session -t 0 runner
    :$ jj git init exact
    :$ jj config set --repo -R exact templates.log 'description'
    :$ sh -c 'msg=$(i=1; while [ "$i" -le 11 ]; do printf "exact %02d\n" "$i"; i=$((i+1)); done) && jj describe -R exact -m "$msg"'
    :$ jj git init overflow
    :$ jj config set --repo -R overflow templates.log 'description'
    :$ sh -c 'msg=$(i=1; while [ "$i" -le 12 ]; do printf "overflow %02d\n" "$i"; i=$((i+1)); done) && jj describe -R overflow -m "$msg"'
    :t new-session -d -s plain "cat"
    :t new-session -d -s ui "sesh cli -r exact -r overflow"
    :t resize-window -t ui:0 -x 120 -y 12
    :pane ui:0.0

This snapshot shows a preview that exactly fits the viewport, so no preview
scrollbar should be visible.

    :k exact down
    :snap -d 2s

This snapshot shows a preview that is one line taller than the viewport, so the
preview scrollbar should now be visible.

    :k C-u overflow down
    :snap

This snapshot shows that once the scrollbar appears, `S-down` scrolls the
preview content by one line.

    :k S-down
    :snap

---
vim: set ft=markdown:
