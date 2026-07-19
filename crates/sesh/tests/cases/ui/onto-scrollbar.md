# Onto scrollbar

This scenario verifies that the onto picker selects and inverts the working-copy
commit, navigates commits independently from fuzzy matching, and scrolls an
overflowing current-repo log.

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
    :$ jj new -R long
    :$ jj describe -R long -m child
    :$ jj edit -R long @-
    :t new-session -d -s plain "cat"
    :t new-session -d -s ui "cd long && sesh -r ../long"
    :t resize-window -t ui:0 -x 90 -y 10
    :pane ui:0.0

Press `C-o` to render the current repo log in the onto picker. The child commit
appears first, but the working-copy commit marked `@` should start selected and
inverted. The scrollbar thumb should remain visible at the top edge.

    :settle -d 2s
    :k C-o
    :snap --color -d 2s "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

Typing an onto query should update the fuzzy model used by rendering. The SVG
snapshot preserves the underlines on the matching characters in the selected
`line 06` commit.

    :k pty6
    :snap --color "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

`Down` should move selection to `line 05` even though it does not match the
query. The match underline stays on `line 06` while the inversion moves down.

    :k Down
    :snap --color "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

`Up` should move selection back to the working-copy commit.

    :k Up
    :snap --color "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

Pressing `Up` twice should move to the child commit, then remain there because it
is the first commit in the view.

    :k Up Up
    :snap --color "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

Repeatedly pressing `Down` past the other end should leave the root commit
selected and scroll it into view.

    :k Down Down Down Down Down Down Down Down Down Down Down Down
    :snap --color "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

---
vim: set ft=markdown:
