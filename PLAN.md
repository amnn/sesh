# Plan: Pick workspace base from the current repo log

## ADR: Use a preview-shaped onto picker backed by the current repo log

### Status

Accepted; implementation in progress.

### Context

`sesh` already shows a cached `jj log` preview for the selected session or
repository. The TODO item is to add `C-o` so users can choose the `onto`
revision used when creating new workspaces.

The desired UX is still to keep the session list visible and use the existing
preview-shaped layout area as the selection surface. However, the onto picker is
not the session preview. Its source must be the current repo context
(`App::repo`), even when the selected session row points at a different repo.
For example, if the current repo is `alpha` and the selected row is `beta`,
`C-o` should show `alpha`'s log.

The onto picker should continue to look like normal `jj log`, while
base-selection mode adds:

- an `onto:` prompt in the existing top prompt location,
- fuzzy match highlighting over log rows without hiding non-matching rows,
- a selected log row highlighted with inverted colours,
- `enter` to choose the selected commit as the base revision.

The session preview cache should remain session-preview-specific. Do not punch
holes through the preview/caching interface or add extra preview-cache feed
calls just to make onto mode work.

### Decision

Use a dedicated `crates/sesh/src/app/onto_picker.rs` module for the picker that
occupies the preview-shaped pane. `crates/sesh/src/app/onto.rs::State` owns the
onto picker state and starts its own worker when entering onto mode:

```rust
onto::State::new(repo.source())
```

The worker loads the current repo's `jj log`, converts ANSI output into
`ratatui::text::Text`, and stores the data needed by the picker in onto-owned
state. The picker models the log as selectable commit blocks, keeps all log
rows visible in their original order, applies fuzzy-match highlighting across
matching blocks, and renders the selected block with a full-row highlight.

`App` remains a coordinator: it creates `onto::State` from the current repo,
renders normal session previews through `preview::Preview`, and delegates onto
mode rendering, query editing, navigation, and base acceptance to `onto`.

The log command still uses jj's built-in compact log template so parsing and
snapshots stay predictable even when user or test configuration overrides
`templates.log`:

```sh
jj log --template builtin_log_compact --color always
```

When accepting a selected log row, resolve that row to a semantic revision
identifier with `jj show`, not with a companion metadata `jj log`:

```sh
jj show -R <repo> -r <selected-change-id> --ignore-working-copy \
  --color never --no-pager \
  --template 'change_id ++ "\t" ++ self.contained_in("trunk()") ++ "\t" ++ local_bookmarks ++ "\t" ++ remote_bookmarks ++ "\n"'
```

Parse the result as:

```text
change_id<TAB>is_trunk<TAB>local_bookmarks<TAB>remote_bookmarks
```

Choose the revision to store in `Repo` using this priority order:

1. `trunk()` when `is_trunk == "true"`
2. first local bookmark
3. first remote bookmark
4. full `change_id`

`trunk()` has highest priority because it preserves the semantic default base
when the selected commit is the configured trunk revision.

### Consequences

- Onto mode renders the current repo context, not the selected session's repo.
- The session preview module/cache remains focused on session previews.
- Entering onto mode may run a separate `jj log` even if the same repo was
  already previewed, but it avoids cache coupling and loads exactly the repo the
  picker should use.
- There is no row-order alignment problem between graphical and non-graph log
  commands because metadata is fetched only for the selected row.
- Base selection needs a best-effort parser for commit header rows in
  `builtin_log_compact` output.
- Rows without a parseable commit/change id remain visible but are not
  acceptable as a base.
- A single `jj show` is run only when accepting a row.

### Alternatives considered

#### Separate full-screen onto picker

Rejected. It hides the session list and duplicates list/picker UI that the
preview-shaped pane already provides.

#### Reuse the session preview cache for onto mode

Rejected. The selected session preview can point at a different repo from the
current repo context. Reusing the session preview cache also forces `App` to
feed cache entries opportunistically for onto mode, which couples two separate
surfaces.

#### Companion metadata `jj log --no-graph`

Rejected for now. It can work, but it requires matching the Nth graphical commit
node in the preview to the Nth metadata row. Using `jj show` after selection is
simpler and avoids line/order alignment concerns.

#### Parse semantic identifiers directly from visible log only

Rejected. The visible log is human-oriented. It is okay for finding a selected
row's short change id, but choosing between trunk, local bookmarks, remote
bookmarks, and full change id should use structured metadata from `jj show`.

#### Use `Picker` for fuzzy matching

Rejected for this mode. `Picker` naturally filters and ranks rows, but onto mode
must keep all preview rows visible in log order. Use
`nucleo_matcher::pattern::Pattern::indices` directly and render all rows.

## UX requirements

### Normal session mode

The existing session UI remains unchanged except the header can show the current
`onto` revision:

```text
session: feature
  3/7 | [C-r] repo: ~/sesh, [C-o] onto: trunk()
▌  sesh/feature ...
────────────────────────────────────────
@  ... selected session jj log preview ...
```

### Base selection mode

Pressing `C-o` when a repo context exists switches the active prompt and uses
the preview-shaped pane as the onto picker, but leaves the session list and its
selection visible:

```text
onto: parser
  3/7 | [C-r] repo: ~/sesh, [C-o] onto: trunk()
▌  sesh/feature ...
────────────────────────────────────────
@  ... current repo jj log with fuzzy highlights ...
```

There is no second prompt inside the pane. The top prompt label changes from
`session:` to `onto:` and edits the revision query. The session query is
preserved and restored when leaving onto mode.

### Key behavior in onto mode

- Text input / `backspace` / `C-u`: edit the revision query.
- `up` / `down`: move selected commit block by one selectable commit.
- `tab`: jump to the next matching commit block.
- `S-tab`: jump to the previous matching commit block.
- `enter`: accept the selected commit block if it has a parsed change id.
- `esc`, `C-g`, `C-c`: cancel onto mode and return to normal session mode.
- `C-o`: optional toggle/cancel back to normal session mode.
- `C-p`: disabled -- can't disable preview when that is the active view ("onto"
  mode also forces the preview pane to be visible).
- `S-up` / `S-down`: scroll the onto pane, if preserving current preview scroll
  behavior is straightforward. Otherwise prefer keeping `up`/`down` as row
  selection and revisit pane scrolling later.

When accepting a row, update `self.repo` with `Repo::with_revision(...)`, return
to normal session mode, and keep the session list query/selection unchanged.

## Implementation plan

### 1. Make log output predictable

Status: Done.

Update `crates/sesh/src/jj.rs::log` so preview and onto-picker log output ignore
user `templates.log` settings:

```rust
Command::new("jj")
    .arg("log")
    .arg("-R")
    .arg(repo)
    .arg("--ignore-working-copy")
    .arg("--no-pager")
    .args(["--config", "ui.graph.style=curved"])
    .args(["--color", "always"])
    .args(["--template", "builtin_log_compact"])
```

The completed implementation uses `--ignore-working-copy`, `--no-pager`, and a
forced curved graph style so onto log parsing does not need to support
user-configured ASCII graph output.

### 2. Add a generic `jj show` helper in `jj.rs`

Status: Done.

Keep `crates/sesh/src/jj.rs` limited to binary-specific command construction
and execution. Do not put onto-picker parsing or revision-priority policy in
`jj.rs`.

The helper runs:

```sh
jj show -R <repo> -r <rev> --ignore-working-copy --color never --no-pager \
  --template <template>
```

and returns stdout on success. Error handling mirrors the other `jj.rs` helpers
and includes stderr when the command fails.

### 3. Add onto state and current-repo log loading

Status: Done for the render-only milestone.

`App` owns `Option<onto::State>` to represent whether onto mode is active.
Entering mode uses the current repo context:

```rust
self.onto = Some(onto::State::new(repo.source()));
```

`onto::State` owns:

- the onto query,
- `picker::State`,
- a retained `component::loader::Loader<picker::Picker>` that loads `jj log` for
  the current repo.

Do not route this through `preview::State` or `PreviewCache`.

### 4. Render the onto pane

Status: Done for the render-only milestone.

`onto::State::draw` renders the retained loader, which delegates to
`picker::Picker` once loading completes. At this milestone, `picker` only
renders the provided text and its scrollbar.

### 5. Segment onto log text into selectable commits

Status: Done for the index-only render milestone.

Do **not** assume every commit is two lines. With `builtin_log_compact`, most
non-root commits are two lines, but root commits, elisions, graph connector
lines, merges, and future jj graph output can vary.

The picker stores the original rendered text and commit metadata in rendered
row order. Each commit records its first rendered row:

```rust
struct Picker {
    text: Text<'static>,
    commits: Vec<Commit>,
}

struct Commit {
    start: usize,
    text: Vec<String>,
    revision_hint: String,
}
```

`Text` remains the rendering source of truth. The commit list carries only
semantic search and selection metadata. Commit height is derived from
`text.len()` when needed.

Parsing approach:

1. Flatten each `Line` into plain text for detection/search.
2. Detect commit header lines in forced-curved `builtin_log_compact` output with
   `^(?:│ )*[@○◆×](?: │)* {2,}(?P<rev>[a-z]+)(?:\s|$)`.
3. A commit starts at a detected header and continues until the next detected
   header.
4. A `~` elision/gap node in any graph lane terminates the current commit and
   remains visible but unindexed.
5. Lines before the first commit header remain visible but unindexed.

### 6. Fuzzy matching without filtering rows

Use `nucleo_matcher::pattern::Pattern` (via the existing `nucleo` re-export is
fine) directly:

```rust
pattern.reparse(&query, CaseMatching::Smart, Normalization::Smart);
pattern.indices(utf32_searchable.slice(..), &mut matcher, &mut indices);
```

Candidate lines identify both their commit's position in `Picker::commits` and
their line within that commit. The commit position supports direct selection
for match navigation, while the local line identifies the rendered row to
highlight:

```rust
struct Candidate {
    commit: usize,
    line: usize,
    text: String,
}
```

Rendering rules:

- all preview lines remain visible,
- matching commit blocks get character-level highlight using existing
  `ui::Highlight`,
- non-matching blocks remain visible unchanged,
- selected commit block is inverted/reversed across the full rendered row width,
- unselectable context lines are never selected but remain visible.

Because selected-row inversion needs to cover trailing whitespace, prefer a
custom preview renderer that writes lines to the buffer and applies row style to
the full `Rect` row before rendering spans. A plain `Paragraph` with reversed
spans may not invert the full row width.

### 7. Navigation behavior

Status: Done.

Maintain selection as an index into selectable commit blocks, not raw terminal
lines.

- On entering onto mode, select the current workspace's working-copy block
  marked `@`, even when it is not the first commit in the rendered log. Fall
  back to the first selectable block when no working-copy block can be parsed.
- `up` / `down`: previous/next selectable block.
- `tab` / `S-tab`: next/previous block whose `score.is_some()` when query is
  non-empty, wrapping at the end/beginning. Both keys are no-ops when the query
  is empty or has no matches.
- If the query changes and the current selection no longer matches, keep the
  selection where it is. `tab`/`S-tab` are the explicit match navigation.

The render pass stores matching commit positions in a `BTreeSet` on picker
state. This keeps navigation synchronized with the fuzzy snapshot that was
actually rendered and gives `tab`/`S-tab` logarithmic next/previous range
lookups instead of scanning the matches.

### 8. Accepting a base row

On `enter` in onto mode:

1. If selected block has no `revision_hint`, no-op and stay in onto mode.
2. Optionally check a metadata cache for `(repo.source(), revision_hint)`.
3. If absent, call `jj::show(repo.source(), revision_hint, BASE_REVISION_TEMPLATE)`
   and parse stdout into `BaseRevisionMetadata` in the onto-picker/onto module.
4. Compute `preferred_revision()`.
5. Update repo context: `repo = repo.with_revision(preferred.to_owned())`.
6. Return to normal session mode.

Base metadata shape:

```rust
const BASE_REVISION_TEMPLATE: &str =
    r#"change_id ++ "\t" ++ self.contained_in("trunk()") ++ "\t" ++ local_bookmarks ++ "\t" ++ remote_bookmarks ++ "\n""#;

struct BaseRevisionMetadata {
    change_id: String,
    is_trunk: bool,
    local_bookmarks: Vec<String>,
    remote_bookmarks: Vec<String>,
}
```

Parser:

- split stdout into exactly four tab-separated fields,
- parse `is_trunk` from `"true"` / `"false"`,
- split bookmark fields with `split_whitespace()`,
- return an error for malformed output.

Priority order:

```rust
impl BaseRevisionMetadata {
    fn preferred_revision(&self) -> &str {
        if self.is_trunk {
            "trunk()"
        } else if let Some(local) = self.local_bookmarks.first() {
            local
        } else if let Some(remote) = self.remote_bookmarks.first() {
            remote
        } else {
            &self.change_id
        }
    }
}
```

If metadata lookup fails, prefer staying in onto mode and showing an inline error
in the pane/header rather than exiting the app. If that is too much for the
first implementation, propagating the error is acceptable but should be called
out in the PR/response.

### 9. Tests and snapshots

Already covered:

- `jj::log` uses `builtin_log_compact` even when test config sets
  `templates.log = "description"`.
- `jj::show` returns real `jj show` output for the base revision template.
- `C-o` changes the top prompt label to `onto:`.
- The session list remains visible/unchanged in onto mode.
- The onto picker renders the current repo context, not the selected row's repo.
- `C-g` cancels onto mode and restores the `session:` prompt/query.
- Typing a query highlights matching preview rows without hiding non-matching
  rows.
- `tab`/`S-tab` jump between matching rows and wrap in rendered order.

Still needed:

- Base-picker metadata parsing handles the `jj show` output format.
- `BaseRevisionMetadata::preferred_revision` priority:
  - trunk wins over all bookmarks,
  - local bookmark wins over remote/change,
  - remote bookmark wins over change,
  - change id fallback.
- `enter` accepts a selected row and updates header `onto` text.

When snapshots change because `jj::log` now forces `builtin_log_compact`, refresh
with `cargo insta test --accept -p sesh` and remove any `.snap.new` artifacts.
Inspect SVG diffs if color/inversion behavior is under test.

### 10. Validation

After implementation:

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo nextest run -p sesh
```

Then run the `nits` subagent using the repo-required fix-first invocation.
