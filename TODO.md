# TODO

## Testing

- [ ] Interaction tests for UI

## Foundation

- [ ] Colorize parts of item text (for example repo name and repo-prefix/glob segment).
  - Keep rendering constraints in mind while using Skim.

- [ ] Detect and display the current repo from `cwd`.
  - Surface this in the UI state so repo-scoped actions can use it.

- [ ] Add a static header that shows shortcuts and current state.
  - Include active filters and key actions so behavior is discoverable.

- [ ] Contract repo path prefixes to initials for compact display.
  - Example: `~/Code/foo/bar` -> `~/C/f/bar`.
  - Apply consistently in list rows while preserving an unambiguous full path elsewhere.

## Session and Repo Lifecycle

- [ ] Selecting an active session should switch to it.
  - Current selection behavior should prioritize fast session attach.

- [ ] Selecting a repo that is not active should create a session for it.

- [ ] Ensure new tmux sessions created from repos store repo association via a tmux user option.
  - Keep the metadata contract stable for discovery and preview logic.

- [ ] Add `C-r` to change repo context (or clear current repo context if already set).
  - This should work consistently whether a repo or session row is selected.

- [ ] Add `C-n` to create a brand-new session/workspace.

- [ ] Add `C-x` to close the selected session.

- [ ] Add `C-d` to delete the selected session/workspace (destroys underlying workspace if applicable).
  - Confirm destructive behavior, if possible.

- [ ] Add `C-t` to pick trunk (the commit used as the base for new workspaces).
  - Define trunk resolution rules for mixed repo types.

## Preview and Workflow Enhancements

- [ ] Add `jj log` preview for repos and for sessions associated with a repo.
  - Non-jj sessions just show their tmux session panes, as they do now.

- [ ] Add a tmux shortcut/entry point to launch this tool.
  - Prefer a discoverable, documented binding and command alias.

- [ ] Investigate and improve debug-build performance.
  - Profile hot paths (preview generation, matching, tmux I/O) and document findings.

## Configuration

- [ ] Add a TOML-based config system.
  - Define config file location(s), defaults, and merge/override precedence.
  - Move hard-coded UI/runtime values (for example preview sizing) into config.

## Longer-Term UI Direction

- [ ] Replace Skim with an in-project ratatui-based renderer.
  - Goal: deterministic preview rendering without ANSI/parser artifacts.
  - Keep fuzzy finding quality and keyboard UX at parity during migration.
