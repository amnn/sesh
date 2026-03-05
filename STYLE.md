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

For `impl` and trait `impl` sections, treat member items (for example methods,
associated consts, and associated types) as entries and apply the same rule:
decreasing visibility, then lexicographic order. Make an exception for
constructors, which should be surfaced at the top of their `impl` block (but
can again be sorted by visibility and then lexicographically).

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

Exception: `use super::*` is recommended inside a nested test (`#[cfg(test)]`)
module.

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

### Constants and Literals

Factor literals out into constants if they are used in multiple places and the
constant would have a clear name that conveys the meaning of the literal, and
not the contents.

Conversely, DO NOT factor literals out into constants if the name of the
constant would change any time the literal's value changes.

### Comments

Non-trivial modules, structs, enums, type aliases, and functions should have
doc comments, with detail appropriate to the item kind:

- Modules: what grouping of logic the module represents, scope boundaries, and
  representative examples where helpful.
- Structs/enums/type aliases: what data is modeled, key invariants, and field
  doc comments where field-level intent is non-obvious.
- Functions/methods: what they do (not how), relevant parameters/returns, and
  any meaningful invariants, preconditions, postconditions, panic conditions,
  or error conditions.

Hard wrap comments at column 100.

### Turbofish

Avoid turbofish operators where the following transformation is possible:

```rust
let x = (..1).collect::<..2>(); -> let x: ..2 = (..1).collect();
```

It's okay to use the turbofish in circumstances where the type cannot be
inferred and there is not a binding that a type annotation can be attached to.

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

### Paths

Avoid building paths using hard-coded separators (e.g., `"/"`). Use
`std::path::PathBuf` and its `Extend` implementation to add multiple components
to preserve the portability of the code.
