# Runner parser error behavior

## Unterminated shell args

This section verifies that shlex failures are preserved as parser errors and emitted as WARNING
callouts by the runner.

:sh "unterminated

## Unknown directive

This section verifies that unknown directives are surfaced as parser errors in transcript output,
rather than being silently ignored.

:unknown abc

## Invalid snap regex

This section verifies that invalid regular expressions in `:snap` become parser errors with
diagnostic detail.

:snap /(unterminated/repl/

---
vim: set ft=markdown:
