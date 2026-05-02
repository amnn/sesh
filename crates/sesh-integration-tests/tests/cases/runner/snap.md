# Runner snap directive behavior

## Captures stable pane content

If repeated pane captures settle to five identical filtered snapshots before
the timeout, the settled capture should be emitted as the snapshot.

    :bins echo sleep python3

    :t new-window -d -n stable 'echo "hello stable"; tmux wait-for -S ready-stable; sleep 10'
    :p 0:stable.0
    :t resize-window -x 80 -y 2 -t 0:stable

    :t wait-for ready-stable
    :snap -c 1 /stable/X

## Paints multiple capture groups only

When a snap filter has multiple capture groups, only captured ranges should be painted and
surrounding literal text should remain unchanged.

    :t new-window -d -n groups 'echo "id=123 user=alice"; tmux wait-for -S ready-groups; sleep 10'
    :p 0:groups.0
    :t resize-window -x 80 -y 2 -t 0:groups

    :t wait-for ready-groups
    :snap -c 1 "/id=([0-9]+) user=([a-z]+)/é"

## Paints nested capture groups once

Nested capture groups overlap; painting should merge overlapping captured ranges so characters are
painted once.

    :t new-window -d -n nested 'echo "token=abcd"; tmux wait-for -S ready-nested; sleep 10'
    :p 0:nested.0
    :t resize-window -x 80 -y 2 -t 0:nested

    :t wait-for ready-nested
    :snap -c 1 /token=(a(bc)d)/é

## Preserves colors when painting filtered output

A colorized snap should apply filter replacement characters on top of the pane cell styles, so the
linked SVG snapshot keeps the replacements colorized across multiple source colors.

    :w scripts/rainbow.py

```python
from subprocess import run

colors = [31, 32, 33, 34, 35, 36, 91, 92, 93]
for color, char in zip(colors, "colorized", strict=True):
    print(f"\033[{color}m{char}", end="")
print("\033[0m", flush=True)
run(["tmux", "wait-for", "-S", "ready-color"], check=True)
```


    :t new-window -d -n color 'python3 scripts/rainbow.py; sleep 10'
    :p 0:color.0
    :t resize-window -x 80 -y 2 -t 0:color

    :t wait-for ready-color
    :snap --color -c 1 /colorized/X

## Settles without emitting a snapshot

The settle directive uses the same settling and filtering rules as snap, but does not append a
terminal block when the pane settles.

    :t new-window -d -n settle 'echo "hello settle"; tmux wait-for -S ready-settle; sleep 10'
    :p 0:settle.0
    :t resize-window -x 80 -y 2 -t 0:settle

    :t wait-for ready-settle
    :settle -c 1 /settle/X

## Warns for unstable pane content

If repeated pane captures do not settle before the timeout, the runner should
emit a warning instead of a snapshot.

    :w scripts/unstable.py

```python
from subprocess import run
from time import sleep

run(["tmux", "wait-for", "-S", "ready-unstable"], check=True)
for i in range(1000000):
    print(i, flush=True)
    sleep(0.005)
```


    :t new-window -d -n unstable 'python3 scripts/unstable.py'
    :p 0:unstable.0
    :t resize-window -x 80 -y 2 -t 0:unstable

    :t wait-for ready-unstable
    :snap -d 200ms

---
vim: set ft=markdown:
