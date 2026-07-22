# TODO

## Session and Repo Lifecycle

- [x] Add `C-o` to pick the `onto` revision used for new workspaces.
  - Label this as `onto:` in the header and user-facing text.
  - Define revision resolution rules for mixed repo types.

- [x] `C-n` to create a session without switching to it (and without closing
  the picker).

- [ ] Find a way to keep the session picker working even if the workspace is
  stale.

- [ ] Rename session

## UI

- [x] Flag and unflag session.
  - Add a mechanism for setting a flag on a session.
  - This causes its pip to remain lit up even if it does not have an alert (but
    with a different colour).
  - Flags must be manually cleared by the user.
  - `C-f` is the setting to toggle flags. It should show up in the header with
    a `flag` or `unflag` label.

- [x] Clearer alert indicator -- the green colour is not bright enough to
  distinguish from the grey.

- [ ] Spinner for long-running operations -- operations on `sui` can take time
  so it would be good to have a visual indicator that something is happening.

## README

- [ ] README: Screenshot/animation
- [ ] README: Feature list
- [ ] README: Contribution

## GitHub

- [ ] List relevant PRs for repo in session list (a relevant PR is one that is
  related to the user in some way -- e.g. they are the author, or they are a
  reviewer, commenter, or mention in the PR).

- [ ] Associate PR information with sessions that have a repo that is attached
  to a PR.

# Appendix: Legend

- [ ] TODO: Not started
- [/] DOING In progress
- [-] DROP: No longer planned or desired
- [x] DONE: Completed
- [^] WAIT: Blocked by an external dependency or prerequisite
- [!] PRIO: High priority or critical path
