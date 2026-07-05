# Onto scrollbar

This scenario verifies that the onto picker renders a scrollbar for a current
repo log that overflows the preview-shaped pane, then applies fuzzy-match
underlines to matching log text. Scrolling is not wired yet, so the first check
only asserts that the scrollbar is present at the top of the pane.

    :bins jj cat python3

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :w scripts/mklog.py

```python
from subprocess import run
from sys import argv

repo, prefix, count = argv[1:]
count = int(count)
for i in range(1, count + 1):
    run(["jj", "describe", "-R", repo, "-m", f"{prefix} {i:02d}"], check=True)
    if i != count:
        run(["jj", "new", "-R", repo], check=True)
```

    :t rename-session -t 0 runner
    :$ jj git init long
    :$ python3 scripts/mklog.py long line 6
    :t new-session -d -s plain "cat"
    :t new-session -d -s ui "cd long && sesh -r ../long"
    :t resize-window -t ui:0 -x 90 -y 10
    :pane ui:0.0

Press `C-o` to render the current repo log in the onto picker. The log is
longer than the preview-shaped pane, so the scrollbar thumb should be visible
at the top edge.

    :settle -d 2s
    :k C-o
    :snap -d 2s "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

Typing an onto query should update the fuzzy model used by rendering. The SVG
snapshot preserves the underlines on the matching characters in the visible
`line 06` description.

    :k pty6
    :snap --color "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

---
vim: set ft=markdown:
