## Validation

Perform all these steps once you are happy with a solution.

- Formatting: `cargo fmt`
- Lints: `cargo clippy -- -D warnings`
- Tests: `cargo nextest run`.

## Code Style

### File Order

Each Rust module should follow the following order:

- Copyright notice
- Module doc comment
- Child modules
- Imports
- Re-exports
- Constants
- Type definitions and aliases
- `impl` blocks
- Trait `impl` blocks
- Module-level functions
- Test module

Entries in each section are in decreasing order of visibility, and then
sorted lexicographically.

If you find that a module is getting too large, and elements pertaining to,
e.g. the same type are spread out too far, that is a sign that this module
needs to be split up.

### Imports

Imports at the top of the file are broken up into three sections, with an
empty line between each:

- `std` imports
- external crate imports
- internal crate imports

`tokio`, `tokio_util`, are examples of external imports.

In each section, there is one import per-line (no nested imports, or wildcard
imports), and the imports are sorted alphabetically. Internal crate imports all
start with `crate::`, (never `super::` or `self::`).

If traits are imported for use (rather than definition), import them `as _`.

Re-exported items (i.e., `pub use ...`) are grouped separately at the bottom of
the import list (with an empty line separating them from the rest of the
imports). Items should only be re-exported from module `A` in another module
`B`, if other modules with access to `B` do not also have access to `A`
(to prevent introducing opportunities for the same item to be imported through
different paths in different places).

### Associated Functions

Member functions that don't accept `self` are called associated functions. Only
use these for constructors (functions that return some variant of `Self`).

Other functions that support the `impl` of a type should be kept as
module-level functions.

### Comments

Non-trivial function should have a doc comment, explaining what the function
does (but not how it does it), details about its parameters and return value,
and any relevant invariants, pre-, post-, panic or error conditions.

Hard wrap comments at column 100.

### Turbofish

Avoid turbofish operators where possible:
```rust
let x = (..1).collect::<..2>(); -> let x: ..2 = (..1).collect();
```

### Strings

Prefer using `.to_owned` instead of `.to_string` when converting from `&str` to
`String` (use of `.to_string` is fine in other circumstances).

### Errors

Use `anyhow` to construct internal errors, and `thiserror` to define structured
error types for public APIs.

- Do not import `anyhow::Result`; write `anyhow::Result` explicitly in type
  signatures.
- Make use of `bail!`, `ensure!`, when appropriate:
```rust
return Err(anyhow!(...)) -> bail!(...)
if !cond { bail!(...) }  -> ensure!(cond, ...)
```
- Make use of `anyhow`'s `Context`:
```rust
.map_err(|e| anyhow!("...: {e}")
  -> .context("...") OR .with_context(|| format!("..."))
```
