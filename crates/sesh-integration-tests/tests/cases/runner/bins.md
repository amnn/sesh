# Runner bins behavior

## One binary succeeds

This section verifies that a single valid binary in `:b` produces a NOTE callout and no
WARNING callout.

:b ls

## Two binaries succeed

This section verifies that exactly two valid binaries in one directive are both reported as
available.

:b ls cat

## Three binaries succeed

This section verifies that 2+ valid binaries are joined correctly in the success message and all
requested binaries are mentioned.

:b ls cat echo

## Missing binary fails

This section verifies that a missing binary produces a WARNING callout that includes the error
message for that exact binary.

:b definitely-not-a-real-binary

## Mixed success and failure

This section verifies that one directive can emit both NOTE and WARNING callouts when it contains
both valid and invalid binaries.

:b ls definitely-not-a-real-binary

## Empty bins directive is a no-op

This section verifies that `:b` with no arguments is accepted and only the raw directive line is
echoed.

:b

---
vim: set ft=markdown:
