# Preview scroll threshold

This scenario verifies the preview scrollbar threshold with the fixed
`builtin_log_compact` preview template:

- when the preview height exactly matches the viewport, no scrollbar is shown,
- when the preview is taller than the viewport, a scrollbar appears,
- and once it appears, the preview can scroll.

The helper script below writes numbered commits into each repo so the test data
is generated consistently.

    :bins jj cat python3

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :w scripts/mklog.py
```python
from subprocess import run
from sys import argv


def main() -> None:
    if len(argv) != 4:
        raise SystemExit("usage: mklog.py <repo> <prefix> <count>")

    repo, prefix, count_text = argv[1:]
    count = int(count_text)
    for i in range(1, count + 1):
        run(["jj", "describe", "-R", repo, "-m", f"{prefix} {i:02d}"], check=True)
        if i != count:
            run(["jj", "new", "-R", repo], check=True)


if __name__ == "__main__":
    main()
```

    :t rename-session -t 0 runner
    :$ jj git init exact
    :$ python3 scripts/mklog.py exact exact 3
    :$ jj git init overflow
    :$ python3 scripts/mklog.py overflow overflow 4
    :t new-session -d -s plain "cat"
    :t new-session -d -s ui "sesh -r exact -r overflow"
    :t resize-window -t ui:0 -x 120 -y 12
    :pane ui:0.0

This snapshot shows a preview that exactly fits the viewport, so no preview
scrollbar should be visible.

    :settle
    :k exact down
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

This snapshot shows a preview that is taller than the viewport, so the preview
scrollbar should now be visible.

    :k C-u overflow down
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

This snapshot shows that once the scrollbar appears, `S-down` scrolls the
preview content by one line.

    :k S-down
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

---
vim: set ft=markdown:
