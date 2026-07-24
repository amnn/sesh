# TODO

## Session and Repo Lifecycle

- [x] Add `C-o` to pick the `onto` revision used for new workspaces.
  - Label this as `onto:` in the header and user-facing text.
  - Define revision resolution rules for mixed repo types.

- [x] `C-n` to create a session without switching to it (and without closing
  the picker).

- [ ] Find a way to keep the session picker working even if the workspace is
  stale.

- [ ] Repo handling after a deletion. Operations that require the repo seem to
  break after the current repo gets deleted.

- [ ] Rename session

## Harnesses

- [ ] Agent progress indicator -- `sesh` exposes commands that agents can call
  to set their status (which in turn set a tmux user option on the underlying
  pane. Harness extensions call these commands to indicate when an agent is
  idle, running, waiting for input, succeeded or failed. `sesh` displays this
  information in the session list.

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
