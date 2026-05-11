# TODO

## Session and Repo Lifecycle

- [ ] `C-x` should not close the entire app -- just trigger a reset of some
  kind.

- [x] Add workspace creation for new repo-backed work.
  - Name is derived from repo name + workspace name.
  - Create a workspace

- [x] Sanitize tmux session names (`:` and `.` are replaced with `_`).

- [x] Apply name deconflicting logic to new sessions, instead of not
  allowing them to be created at all.

- [ ] New session preview should show a preview of `jj` log at the base repo,
  if applicable (if a repo is set).

- [ ] Add `C-d` to delete the selected session/workspace.
  - Destroy the underlying workspace too, if applicable.
  - Confirm destructive behavior, if possible.

- [ ] Add `C-b` to pick the base commit used for new workspaces.
  - Label this as `base:` in the header and user-facing text.
  - Define base resolution rules for mixed repo types.

## GitHub

- [ ] List relevant PRs for repo in session list (a relevant PR is one that is
  related to the user in some way -- e.g. they are the author, or they are a
  reviewer, commenter, or mention in the PR).

- [ ] Associate PR information with sessions that have a repo that is attached
  to a PR.

## Configuration

- [ ] Add dedicated `help` output sections for key bindings and configuration.
  - Include `M-r` to clear/reset the repo.

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
