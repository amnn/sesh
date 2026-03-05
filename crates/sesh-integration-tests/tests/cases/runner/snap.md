# Runner snap directive behavior

## Captures stable pane content

If repeated pane captures converge, the dominant filtered capture should be
emitted as the snapshot.

:bins echo sleep

:t new-window -d -n stable 'echo "hello stable"; tmux wait-for -S ready-stable; sleep 10'
:p 0:stable.0

:t wait-for ready-stable
:snap /stable/X

## Paints multiple capture groups only

When a snap filter has multiple capture groups, only captured ranges should be painted and
surrounding literal text should remain unchanged.

:t new-window -d -n groups 'echo "id=123 user=alice"; tmux wait-for -S ready-groups; sleep 10'
:p 0:groups.0

:t wait-for ready-groups
:snap "/id=([0-9]+) user=([a-z]+)/👩🏽‍💻"

## Paints nested capture groups once

Nested capture groups overlap; painting should merge overlapping captured ranges so characters are
painted once.

:t new-window -d -n nested 'echo "token=abcd"; tmux wait-for -S ready-nested; sleep 10'
:p 0:nested.0

:t wait-for ready-nested
:snap /token=(a(bc)d)/👩🏽‍💻

## Warns for unstable pane content

If repeated pane captures do not have a dominant value (>75%), the runner should emit a warning
instead of a snapshot.

:bins python3

:t new-window -d -n unstable 'python3 -c "import itertools, time; [print(i, flush=True) or time.sleep(0.005) for i in itertools.count()]"'
:p 0:unstable.0

:$ sleep 0.1
:snap

---
vim: set ft=markdown:
