# Runner parser error behavior

## Unterminated shell args

Shlex failures should be preserved as parser errors and emitted as WARNING callouts by the
runner.

    :sh "unterminated

## Unknown directive

Unknown directives should be surfaced as parser errors in transcript output, rather than being
silently ignored.

    :unknown abc

## Invalid snap regex

Invalid regular expressions in `:snap` should become parser errors with diagnostic detail.

    :snap /(unterminated/x

## Invalid shift-modified text key

Shift modifier should not apply directly to text keys.

    :keys S-a

## Invalid shift-modified non-shiftable key

Shift modifier should only apply to arrow keys.

    :keys S-enter

    :keys S-tab

---
vim: set ft=markdown:
