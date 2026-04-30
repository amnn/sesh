# Preview scroll threshold

This scenario verifies the preview scrollbar threshold. In this fixture, `jj`
still renders the graph markers, so the viewport-fitting case uses 11 content
lines plus the trailing root marker line:

- when the preview height exactly matches the viewport, no scrollbar is shown,
- when the preview is one line longer, a scrollbar appears,
- and once it appears, the preview can scroll.

The test configures `jj log` to show only commit descriptions so the preview is
easy to read in snapshots.

The helper script below writes numbered multiline descriptions into each repo so
the test data is generated consistently.

    :bins jj cat python3

    :w scripts/mklog.py
```python
from subprocess import run
from sys import argv


def main() -> None:
    if len(argv) != 4:
        raise SystemExit("usage: mklog.py <repo> <prefix> <count>")

    repo, prefix, count_text = argv[1:]
    count = int(count_text)
    message = "\n".join(f"{prefix} {i:02d}" for i in range(1, count + 1))
    run(["jj", "describe", "-R", repo, "-m", message], check=True)


if __name__ == "__main__":
    main()
```

    :t rename-session -t 0 runner
    :$ jj git init exact
    :$ jj config set --repo -R exact templates.log 'description'
    :$ python3 scripts/mklog.py exact exact 11
    :$ jj git init overflow
    :$ jj config set --repo -R overflow templates.log 'description'
    :$ python3 scripts/mklog.py overflow overflow 12
    :t new-session -d -s plain "cat"
    :t new-session -d -s ui "sesh -r exact -r overflow"
    :t resize-window -t ui:0 -x 120 -y 12
    :pane ui:0.0

This snapshot shows a preview that exactly fits the viewport, so no preview
scrollbar should be visible.

    :k exact down
    :snap

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
