# TODO

## Rendering

- [ ] Highlight the matched portion of the item.

- [ ] Render the tmux pip green when the session has an undismissed bell/notification.

- [ ] Expand contracted portion of repo paths when they are matched against.

- [ ] Scroll the contents of session list items so that the last matched
  character is visible.

- [x] Re-adjust frame split:
  - The preview should be given 80 characters
  - If the overall screen real-estate is not at least 140 characters, switch to
    a vertical stacked view.

## Session and Repo Lifecycle

- [ ] Set-up windows in new session.
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
