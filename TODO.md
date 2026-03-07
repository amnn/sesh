# TODO

## Agents

- [x] Validate nits subagent: It seems to always try to run a `nits` binary,
  rather than performing an analysis of touched files.
- [x] Reflection skill: Learn from past sessions and pull requests to update
  repo context, skills, and subagent guidance.
- [x] Truth seeking behavior: Prompt the agent to not assume, validate
  assumptions, cite sources.
- [x] Kickoff skill: Add a `kickoff` skill that instructs the agent to check
  `TODO.md` and choose the next most appropriate task to work on.

## Testing

- [ ] Interaction tests for UI
  - [x] Modifier keys for `:k` directive.
  - [x] Review `runner` module.
  - [x] Snapshot filters paint pattern over with single character.
  - [x] Snapshot filters paint pattern supports grapheme clusters.
  - [x] Fix tmux control client leak.
  - [x] Standardize shell, prompt used by tests (don't use user's default
    shell).
  - [x] Configurable session width/height
  - [ ] Test: Basic session.
  - [ ] Test: Session with repo attached.
  - [ ] Test: Session with multiple workspaces.

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

- [ ] Add `C-d` to delete the selected session/workspace.
  - Destroy the underlying workspace too, if applicable.
  - Confirm destructive behavior, if possible.

- [ ] Add `C-t` to pick trunk (the commit used as the base for new workspaces).
  - Define trunk resolution rules for mixed repo types.

## Preview and Workflow Enhancements

- [x] Remove tmux pane-capture preview path that caused Skim rendering artifacts.
  - Preview now uses `jj log` text output instead of terminal pane re-rendering.

- [x] Make the preview window show a summary of the `jj log` for the workspace
  that would be visited, e.g. `jj log --color always`.
  - [ ] This preview should be optional (toggleable/configurable).
  - [x] Applies to repo entries and sessions associated with a repo/workspace.
  - [x] If a selected tmux session has no repo/workspace context, show no preview.

- [ ] Add a tmux shortcut/entry point to launch this tool.

- [ ] Investigate and improve debug-build performance.
  - Profile hot paths (preview generation, matching, tmux I/O) and document findings.

## Configuration

- [ ] Add a TOML-based config system.
  - Define config file location(s), defaults, and merge/override precedence.
  - Move hard-coded UI/runtime values (for example preview sizing) into config.
