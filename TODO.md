# TODO

## Foundation

- [x] Switch to Nucleo and Ratatui based renderer

- [x] `PreviewCache` uses `Utf32String` to store cache keys, to avoid
  re-computing item text to fetch previews.

- [x] Lock down modules: Make all modules in `lib.rs` private, and re-export
  relevant items at the top level.

- [x] Colorize parts of item text (for example repo name and repo-prefix/glob
  segment).
  - [x] Shortcut keys
  - [x] Preview text
  - [x] Selected line symbol
  - [x] Prompt
  - [x] Repo Paths (dim everything apart from the basename)

- [x] Detect and display the current repo from `cwd`.
  - Surface this in the UI state so repo-scoped actions can use it.

- [x] Add a static header that shows shortcuts and current state.
  - [x] `C-r` now sets repo context from the selected repo or repo-backed session.
  - [x] Include active filters and key actions so behavior is discoverable.

- [x] Contract repo path prefixes to initials for compact display.
  - Example: `~/Code/foo/bar` -> `~/C/f/bar`.
  - Apply consistently in list rows while preserving an unambiguous full path
    elsewhere.

## Rendering

- [x] Separate the session list and the preview with a scroll bar.

- [x] Scroll bar for preview window

- [ ] Highlight the matched portion of the item.

- [ ] Render the tmux pip green when the session has an undismissed bell/notification.

- [ ] Expand contracted portion of repo paths when they are matched against.

- [ ] Scroll the contents of session list items so that the last matched
  character is visible.

## Session and Repo Lifecycle

- [x] Switch to the selected active session.
  - Current selection behavior should prioritize fast session attach.

- [x] Create a session when selecting an inactive repo.

- [x] Ensure new tmux sessions created from repos store repo association via a
  tmux user option.
  - Keep the metadata contract stable for discovery and preview logic.

- [x] Add `C-r` to set repo context from the selected repo or repo-backed
  session.
  - Set-only semantics: the current repo may begin empty, but `C-r` does not
    clear it back to none.
  - This should work consistently whether a repo or session row is selected.

- [x] Add `C-n` to create a brand-new session.

- [ ] Add workspace creation for new repo-backed work.

- [ ] Add `C-x` to close the selected session.

- [ ] Add `C-d` to delete the selected session/workspace.
  - Destroy the underlying workspace too, if applicable.
  - Confirm destructive behavior, if possible.

- [ ] Add `C-t` to pick trunk (the commit used as the base for new workspaces).
  - Define trunk resolution rules for mixed repo types.

## Preview and Workflow Enhancements

- [x] Make the preview window optional (toggleable/configurable).
  - `C-p` toggles the preview window at runtime.
  - The preview already shows a `jj log` summary for repo entries and
    repo-associated sessions.

- [x] Render previews asynchronously in the background.
  - Avoid doing preview preparation on the row-selection interaction path.
  - Warm previews ahead of selection so slower `jj log` previews are ready when
    the cursor moves.

- [ ] Add a tmux shortcut/entry point to launch this tool.

- [ ] Investigate and improve debug-build performance.
  - Profile hot paths (preview generation, matching, tmux I/O) and document
    findings.

## GitHub

- [ ] List relevant PRs for repo in session list (a relevant PR is one that is
  related to the user in some way -- e.g. they are the author, or they are a
  reviewer, commenter, or mention in the PR).

- [ ] Associate PR information with sessions that have a repo that is attached
  to a PR.

## Configuration

- [ ] Add a TOML-based config system.
  - Define config file location(s), defaults, and merge/override precedence.
  - Move hard-coded UI/runtime values (for example preview sizing) into config.

# Appendix: Legend

- [ ] TODO: Not started
- [/] DOING In progress
- [-] DROP: No longer planned or desired
- [x] DONE: Completed
- [^] WAIT: Blocked by an external dependency or prerequisite
- [!] PRIO: High priority or critical path
