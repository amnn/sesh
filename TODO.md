# TODO

## Foundation

- [ ] Colorize parts of item text (for example repo name and repo-prefix/glob
  segment).
  - Keep rendering constraints in mind while using Skim.

- [x] Detect and display the current repo from `cwd`.
  - Surface this in the UI state so repo-scoped actions can use it.

- [ ] Add a static header that shows shortcuts and current state.
  - `C-r` now sets repo context from the selected repo or repo-backed session.
  - Include active filters and key actions so behavior is discoverable.

- [ ] Contract repo path prefixes to initials for compact display.
  - Example: `~/Code/foo/bar` -> `~/C/f/bar`.
  - Apply consistently in list rows while preserving an unambiguous full path
    elsewhere.

## Session and Repo Lifecycle

- [ ] Switch to the selected active session.
  - Current selection behavior should prioritize fast session attach.

- [!] Create a session when selecting an inactive repo.

- [!] Ensure new tmux sessions created from repos store repo association via a
  tmux user option.
  - Keep the metadata contract stable for discovery and preview logic.

- [x] Add `C-r` to set repo context from the selected repo or repo-backed
  session.
  - Set-only semantics: the current repo may begin empty, but `C-r` does not
    clear it back to none.
  - This should work consistently whether a repo or session row is selected.

- [ ] Add `C-n` to create a brand-new session/workspace.

- [ ] Add `C-x` to close the selected session.

- [ ] Add `C-d` to delete the selected session/workspace.
  - Destroy the underlying workspace too, if applicable.
  - Confirm destructive behavior, if possible.

- [ ] Add `C-t` to pick trunk (the commit used as the base for new workspaces).
  - Define trunk resolution rules for mixed repo types.

## Preview and Workflow Enhancements

- [ ] Make the preview window optional (toggleable/configurable).
  - The preview already shows a `jj log` summary for repo entries and
    repo-associated sessions.

- [ ] Render previews asynchronously in the background.
  - Avoid doing preview preparation on the row-selection interaction path.
  - Warm previews ahead of selection so slower `jj log` previews are ready when
    the cursor moves.

- [ ] Add a tmux shortcut/entry point to launch this tool.

- [ ] Investigate and improve debug-build performance.
  - Profile hot paths (preview generation, matching, tmux I/O) and document
    findings.

## Configuration

- [ ] Add a TOML-based config system.
  - Define config file location(s), defaults, and merge/override precedence.
  - Move hard-coded UI/runtime values (for example preview sizing) into config.

# Appendix: Legend

- [ ] TODO: No started
- [/] DOING In progress
- [-] DROP: No longer planned or desired
- [x] DONE: Completed
- [^] WAIT: Blocked by an external dependency or prerequisite
- [!] PRIO: High priority or critical path
