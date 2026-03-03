# Runner snap directive behavior

## Captures stable pane content

If repeated pane captures converge, the dominant filtered capture should be
emitted as the snapshot.

:bins echo sleep

:t new-window -d -n stable 'echo "hello stable"; sleep 10'
:p 0:stable.0

:$ sleep 0.1
:snap /stable/STABLE/

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
