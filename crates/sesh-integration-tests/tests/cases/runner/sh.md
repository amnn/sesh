# Runner shell directive behavior

## Successful shell command prints stdout

Successful shell commands should append an exit annotation and then write stdout inside a fenced
code block.

:b echo

:$ echo hello from sh

## Command not in bins fails to execute

A command that exists on the host but has not been added via `:b` should fail to spawn in the
isolated environment and produce a WARNING callout.

:$ cat

## Failing shell command prints stderr

Failing shell commands should append a non-zero exit annotation and then write stderr inside a
fenced code block.

:b sh

:$ sh -c "printf 'hello from stderr\n' >&2; exit 1"

## Command writes both streams and fails

When a command exits non-zero and writes both streams, stdout should be shown first and stderr
should be shown second.

:b sh

:$ sh -c "printf 'hello from stdout\n'; printf 'hello from stderr\n' >&2; exit 7"

## Command writes both streams and succeeds

When a command exits zero and writes both streams, only stdout should be shown in the transcript.

:b sh

:$ sh -c "printf 'hello from stdout\n'; printf 'hello from stderr\n' >&2; exit 0"

## Successful shell command can be silent

Successful shell commands without stdout should only produce the annotated raw line.

:b true

:$ true

---
vim: set ft=markdown:
