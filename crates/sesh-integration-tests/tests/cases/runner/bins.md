# Runner bins behavior

## One binary succeeds

A single valid binary in `:b` should produce a NOTE callout and no WARNING callout.

    :b ls

## Two binaries succeed

Exactly two valid binaries in one directive should both be reported as available.

    :b ls cat

## Three binaries succeed

With 2+ valid binaries, the success message should join them correctly and mention every one.

    :b ls cat echo

## Missing binary fails

A missing binary should produce a WARNING callout that includes an error for that exact binary.

    :b definitely-not-a-real-binary

## Multiple missing binaries fail

Multiple invalid binaries in one directive should produce multiple WARNING callouts.

    :b definitely-not-a-real-binary another-not-a-real-binary

## Mixed success and failure

One directive should emit both NOTE and WARNING callouts when it mixes valid and invalid binaries.

    :b ls definitely-not-a-real-binary

## Mixed multiple success and multiple failure

One directive with multiple valid and invalid binaries should emit one NOTE and multiple WARNING
callouts.

    :b ls cat definitely-not-a-real-binary another-not-a-real-binary

## Empty bins directive is a no-op

`:b` with no arguments should be accepted, and only the raw directive line should be echoed.

    :b

---
vim: set ft=markdown:
