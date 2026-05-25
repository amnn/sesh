# Plan: Pick workspace base from the existing preview log

## ADR: Use the existing preview log as the onto picker surface

### Status

Proposed.

### Context

`sesh` already shows a cached `jj log` preview for the selected session or
repository. The TODO item is to add `C-o` so users can choose the `onto`
revision used when creating new workspaces. Earlier ideas considered opening a
separate revision picker list, but the desired UX is to keep the session list
visible and turn the preview pane itself into the selection surface.

The preview should continue to look like normal `jj log`, while base-selection
mode adds:

- an `onto:` prompt in the existing top prompt location,
- fuzzy match highlighting over log rows without hiding non-matching rows,
- a selected log row highlighted with inverted colours,
- `enter` to choose the selected commit as the base revision.

We must keep the existing session preview cache alive while entering/leaving
onto mode. Do not throw away cached session previews when switching modes.

### Decision

Use the existing cached preview output for display. Force that preview output to
use jj's built-in compact log template so it is predictable even when user or
test configuration overrides `templates.log`:

```sh
jj log --template builtin_log_compact --color always
```

When accepting a selected preview row, resolve that row to a semantic revision
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

- The UI reuses exactly the same log content users already see in the preview.
- There is no row-order alignment problem between graphical and non-graph log
  commands because metadata is fetched only for the selected row.
- Base selection needs a best-effort parser for commit header rows in
  `builtin_log_compact` output.
- Rows without a parseable commit/change id remain visible but are not
  acceptable as a base.
- A single `jj show` is run only when accepting a row. Optionally cache those
  results by `(repo, selected-change-id)`.

### Alternatives considered

#### Separate full-screen onto picker

Rejected. It hides the session list and duplicates list/picker UI that the
preview already provides.

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
@  ... existing jj log preview ...
```

### Base selection mode

Pressing `C-o` when a repo context exists switches the active prompt and preview
behavior, but leaves the session list and its selection visible:

```text
onto: parser
  3/7 | [C-r] repo: ~/sesh, [C-o] onto: trunk()
▌  sesh/feature ...
────────────────────────────────────────
@  ... existing jj log preview with fuzzy highlights ...
```

There is no second prompt inside the preview. The top prompt label changes from
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
- `C-p`: continue to toggle preview visibility.
- `S-up` / `S-down`: scroll preview, if preserving current preview scroll
  behavior is straightforward. Otherwise prefer keeping `up`/`down` as row
  selection and revisit preview scrolling later.

When accepting a row, update `self.repo` with `Repo::with_revision(...)`, return
to normal session mode, and keep the session list query/selection unchanged.

## Implementation plan

### 1. Make preview log output predictable

Status: Done.

Update `crates/sesh/src/jj.rs::log` so preview output ignores user
`templates.log` settings:

```rust
Command::new("jj")
    .arg("log")
    .arg("-R")
    .arg(repo)
    .arg("--ignore-working-copy")
    .arg("--no-pager")
    .args(["--color", "always"])
    .args(["--template", "builtin_log_compact"])
```

The completed implementation uses `--ignore-working-copy` and `--no-pager`.

### 2. Add a generic `jj show` helper in `jj.rs`

Status: Done.

Keep `crates/sesh/src/jj.rs` limited to binary-specific command construction
and execution. Do not put onto-picker parsing or revision-priority policy in
`jj.rs`.

Add a generic helper, for example:

```rust
pub async fn show(repo: &Path, rev: &str, template: &str) -> anyhow::Result<String>;
```

It should run:

```sh
jj show -R <repo> -r <rev> --ignore-working-copy --color never --no-pager \
  --template <template>
```

and return stdout on success. Error handling should mirror the other `jj.rs`
helpers and include stderr when the command fails.

Put onto-picker-specific metadata in the onto/session-picker module, not in
`jj.rs`, for example:

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

- call `jj::show(repo, rev, BASE_REVISION_TEMPLATE)`,
- split stdout into exactly four tab-separated fields,
- parse `is_trunk` from `"true"` / `"false"`,
- split bookmark fields with `split_whitespace()`,
- return an error for malformed output.

Add an onto-picker-owned method:

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

Unit-test the parser and priority order without shelling out. Add one
integration-style `jj` test for `jj::show` with the base revision template if
feasible.

### 3. Expose cached preview text for onto mode

Status: Done.

The current preview cache stores rendered `ratatui::text::Text`. Base mode
should read from that cache instead of running a separate display command.

Add a narrow method to `crates/sesh/src/app/preview.rs` such as:

```rust
impl State {
    pub(super) fn cached(
        &self,
        key: &Option<PathBuf>,
    ) -> Option<Arc<anyhow::Result<Text<'static>>>>;
}
```

Keep cache ownership inside preview state so normal previews are not recreated
when entering/leaving onto mode. Callers are responsible for deciding which
preview key to inspect and for rendering empty/loading/error states from the
returned cache entry.

### 4. Add onto state to session picker UI

Status: Done for initial mode/query scaffolding.

Keep `App` as a coordinator. Do not move base-specific rendering and key logic
into a large `App::handle_key`/`App::draw` branch.

The initial implementation adds `crates/sesh/src/app/onto.rs` with query state
only:

```rust
struct State {
    query: String,
}
```

`App` owns `Option<onto::State>` to represent whether onto mode is active. While
active, the top prompt label is `onto:`, text input edits the revision query,
and cancel/toggle keys leave onto mode. Selection, fuzzy matching, and metadata
cache fields should be added once selectable preview blocks exist.

`App`/session-mode controller should continue to delegate:

- prompt label/query selection,
- preview rendering,
- base-mode key handling,
- accept/cancel transitions.

### 5. Segment cached preview text into selectable commit blocks

Do **not** assume every commit is two lines. With `builtin_log_compact`, most
non-root commits are two lines, but root commits, elisions, graph connector
lines, merges, and future jj graph output can vary.

Represent the preview as blocks:

```rust
struct CommitBlock {
    lines: Vec<Line<'static>>,
    searchable: String,
    revision_hint: Option<String>,
}
```

Parsing approach:

1. Flatten each `Line` into plain text for detection/search.
2. Detect commit header lines in `builtin_log_compact` output.
3. A commit block starts at a detected header and continues until the next
   detected header.
4. Lines before the first commit header or between blocks without a detected
   header remain visible as unselectable context.

Header detection can be heuristic because `builtin_log_compact` is fixed. The
first token after a graph node marker is the short change id. For current jj,
examples look like:

```text
@  wromuovm user date refs commitid
│ ○  abcdefgh user date refs commitid
◆  zzzzzzzz root() 00000000
```

Implement a small parser that identifies graph node markers (`@`, `○`, `◆`,
`×`, and ascii equivalents if tests require them) and extracts the following
alphanumeric id as `revision_hint`. Keep this parser isolated and unit-tested.
If no id can be parsed, the line is not selectable.

### 6. Fuzzy matching without filtering rows

Use `nucleo_matcher::pattern::Pattern` (via the existing `nucleo` re-export is
fine) directly:

```rust
pattern.reparse(&query, CaseMatching::Smart, Normalization::Smart);
pattern.indices(utf32_searchable.slice(..), &mut matcher, &mut indices);
```

For each commit block store:

```rust
struct MatchState {
    score: Option<u32>,
    indices: Vec<u32>,
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

Maintain selection as an index into selectable commit blocks, not raw terminal
lines.

- On entering onto mode, select the first visible selectable block, preferably
  the block corresponding to the current preview's top/current row if easy.
- `up` / `down`: previous/next selectable block.
- `tab` / `S-tab`: next/previous block whose `score.is_some()` when query is
  non-empty. If query is empty, either no-op or behave like down/up; pick one
  and test it.
- If the query changes and the current selection no longer matches, keep the
  selection where it is. `tab`/`S-tab` are the explicit match navigation.

### 8. Accepting a base row

On `enter` in onto mode:

1. If selected block has no `revision_hint`, no-op and stay in onto mode.
2. Check `metadata_cache` for `(repo.source(), revision_hint)`.
3. If absent, call `jj::show(repo.source(), revision_hint, BASE_REVISION_TEMPLATE)`
   and parse the stdout into `BaseRevisionMetadata` in the onto-picker module.
4. Compute `preferred_revision()`.
5. Update repo context: `repo = repo.with_revision(preferred.to_owned())`.
6. Return to normal session mode.

If metadata lookup fails, prefer staying in onto mode and showing an inline error
in the preview/header rather than exiting the app. If that is too much for the
first implementation, propagating the error is acceptable but should be called
out in the PR/response.

### 9. Tests and snapshots

Add or update tests for:

1. `jj::log` uses `builtin_log_compact` even when test config sets
   `templates.log = "description"`.
2. `jj::show` returns real `jj show` output for the base revision template.
3. Base-picker metadata parsing handles the `jj show` output format.
4. `BaseRevisionMetadata::preferred_revision` priority:
   - trunk wins over all bookmarks,
   - local bookmark wins over remote/change,
   - remote bookmark wins over change,
   - change id fallback.
5. Base mode UI:
   - `C-o` changes the top prompt label to `onto:`.
   - session list remains visible/unchanged.
   - preview log remains visible.
   - typing a query highlights matching preview rows without hiding
     non-matching rows.
   - `tab`/`S-tab` jump between matching rows.
   - `esc` cancels and restores the `session:` prompt/query.
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
