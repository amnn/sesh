## Code Style

### Imports
Imports at the top of the file are broken up into three sections, with an
empty line between each:

- `std` imports
- external crate imports
- internal crate imports

In each section, there is one import per-line (no nested imports, or wildcard
imports), and the imports are sorted alphabetically. Internal crate imports all
start with `crate::`, (never `super::` or `self::`).

Re-exported items (i.e., `pub use ...`) are grouped separately at the bottom of
the import list (with an empty line separating them from the rest of the
imports). Items should only be re-exported from module `A` in another module
`B`, if other modules with access to `B` do not also have access to `A`
(to prevent introducing opportunities for the same item to be imported through
different paths in different places).
