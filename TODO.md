# TODO

## Testing

- [ ] Take SVG snapshots of snapped tmux frames for visual regression testing.

## Rendering

- [ ] Get rid of `popup` command (it is redundant).

- [ ] Render the tmux pip green when the session has an undismissed bell/notification.
  - [ ] Track bell state per tmux session.
  - [ ] Switch to first Window in session with bell when session is selected.
  - [ ] Refactor: Move session switching logic onto `Session` type.

- [ ] Generate a line/entry for the "new" session instead of having a dedicated
  key combination to create it.
  - Introduce key combinations to go to the top or bottom of the session list
    instead (this session will go at the top).
  - Leave an empty row (visual spacer) at the top of the list if a valid
    new session cannot be created, to prevent the other results from jumping
    around.

- [ ] Highlight the matched portion of the item.

- [ ] Expand contracted portion of repo paths when they are matched against.

- [ ] Scroll the contents of session list items so that the last matched
  character is visible.

- [x] Re-adjust frame split:
  - The preview should be given 80 characters
  - If the overall screen real-estate is not at least 140 characters, switch to
    a vertical stacked view.

## Matching

- [ ] Split fuzzy matching over multiple columns
  - State (bells), name, repo, PR info etc should sent as separate columns.
  - Send truncated repo path even though the rendering is compacted.

## Session and Repo Lifecycle

- [x] Set-up windows in new session.
  - Editor
  - Agent Harness
  - Shell

- [x] Add `C-x` to close the selected session.

- [ ] Add `C-d` to delete the selected session/workspace.
  - Destroy the underlying workspace too, if applicable.
  - Confirm destructive behavior, if possible.

- [ ] Add `C-t` to pick trunk (the commit used as the base for new workspaces).
  - Define trunk resolution rules for mixed repo types.

- [ ] Add workspace creation for new repo-backed work.
  - Name is derived from repo name + workspace name.
  - Create a workspace

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

- [x] Configure new session set-up

- [ ] Configure previews

- [ ] Configure repo detection

- [ ] Configure listing trunks

- [ ] Configure workspace creation
  - [ ] How to create a new workspace
  - [ ] How to name workspace directories
  - [ ] How to name workspace sessions
  - [ ] Where to create workspace directories

# Appendix: Legend

- [ ] TODO: Not started
- [/] DOING In progress
- [-] DROP: No longer planned or desired
- [x] DONE: Completed
- [^] WAIT: Blocked by an external dependency or prerequisite
- [!] PRIO: High priority or critical path
