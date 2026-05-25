# Preview scroll

This scenario verifies the preview scrollbar behavior for a long repo-backed
preview: it should start at the top and reach the bottom after scrolling.

The test creates a chain of numbered commits. With the fixed
`builtin_log_compact` preview template, each commit contributes a compact header
and description line, keeping the preview stable while making the scrollbar
movement easy to read in snapshots.

The helper script below writes numbered commits into a repo so these
long-preview fixtures stay compact and readable.

    :bins jj cat python3

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :w scripts/mklog.py
```python
from subprocess import run
from sys import argv

repo, prefix, count_text = argv[1:]
count = int(count_text)
for i in range(1, count + 1):
    run(["jj", "describe", "-R", repo, "-m", f"{prefix} {i:02d}"], check=True)
    if i != count:
        run(["jj", "new", "-R", repo], check=True)
```

    :t rename-session -t 0 runner
    :$ jj git init long
    :$ python3 scripts/mklog.py long line 9
    :t new-session -d -s plain "cat"
    :t new-session -d -s ui "sesh -r long"
    :t resize-window -t ui:0 -x 120 -y 12
    :pane ui:0.0

This snapshot shows the long preview at its initial position. The preview
scrollbar thumb should start at the top of the scroll area.

    :settle
    :k long down
    :snap -d 2s "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

This snapshot shows the preview after repeated `S-down` presses. The preview
scrollbar thumb should reach the bottom of the scroll area.

    :k S-down S-down S-down S-down S-down S-down S-down S-down S-down S-down S-down S-down S-down S-down S-down
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

---
vim: set ft=markdown:
