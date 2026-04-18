# Preview scroll

This scenario verifies the preview scrollbar behavior for a long repo-backed
preview: it should start at the top and reach the bottom after scrolling.

The test configures `jj log` to show only commit descriptions, and uses a
single commit with a multiline description. That keeps the preview stable while
making the scrollbar movement easy to read in snapshots.

The helper script below writes a numbered multiline description into a repo so
these long-preview fixtures stay compact and readable.

    :bins jj cat python3

    :w scripts/mklog.py
```python
from subprocess import run
from sys import argv

repo, prefix, count_text = argv[1:]
count = int(count_text)
message = "\n".join(f"{prefix} {i:02d}" for i in range(1, count + 1))
run(["jj", "describe", "-R", repo, "-m", message], check=True)
```

    :t rename-session -t 0 runner
    :$ jj git init long
    :$ jj config set --repo -R long templates.log 'description'
    :$ python3 scripts/mklog.py long line 18
    :t new-session -d -s plain "cat"
    :t new-session -d -s ui "sesh cli -r long"
    :t resize-window -t ui:0 -x 120 -y 12
    :pane ui:0.0

This snapshot shows the long preview at its initial position. The preview
scrollbar thumb should start at the top of the scroll area.

    :k long down
    :snap -d 2s

This snapshot shows the preview after repeated `S-down` presses. The preview
scrollbar thumb should reach the bottom of the scroll area.

    :k S-down S-down S-down S-down S-down S-down S-down S-down S-down S-down S-down S-down S-down S-down S-down
    :snap

---
vim: set ft=markdown:
