# sesh: tmux session switcher

A tmux-native session switcher, for navigating between and opening new sessions
based on jujutsu (jj) repositories and workspaces. The session switcher
supports opening new sessions:

- based on a jj repository
  - ...and existing workspace
  - ...and base revision (to create a new workspace)
  - ...on its own.
- based on a custom name (and no repository), to create a simple tmux session.

## Configuration
The switcher is configured via a configuration file at
`~/.config/sesh/config.toml`, containing the following properties:

- `repo.globs`: A list of glob patterns to locate jj repositories.
- `workspace.template`: A template for naming new workspaces. This can be
  a relative path that ends in a directory name that contains the `{repo}`
  and `{name}` placeholders. `{repo}` is the repo basename, and `{name}`
  must not contain `/`.
  - **Default**: `../{repo}.{name}`
- `session.template`: A template for configuring how to set up a new tmux
  session.
- `session.name`: A template for configuring the name of the tmux session.

## State
The session switcher maintains the following state:

- When the switcher is open, it remembers:
  - The currently selected repository (if any). Defaults to the closest
    containing jj repo root to the current working directory of the active tmux
    pane.
- The currently selected base revision, if there is a selected repository.
  Defaults to `trunk()`, and resets if the selected repository changes. The
  base picker lists bookmarks (including `origin/*`) and a `trunk()` entry.
- On each open session, tmux metadata is added to indicate whether the session
  corresponds to a repository/workspace, or is plain.
  - This is used by the session switcher to advertise metadata about
    existing sessions.

## UX
The session switcher opens a tmux pop-over when a tmux kebinding is pressed.
The popover includes a header, a fuzzy finder and a pane previewing the
selected session.

### Header
The fuzzy finder includes a header with the following information:

- Currently selected repo (if any).
- Currently selected base (if there is a repo).
- Hints for keybindings:
  - `C-r` to change repo (next to the current repo).
  - `C-b` to change base revision (next to the current base).
  - `C-n` to create a new session from the current query.
  - `C-x` to clean a session.

### Candidate Sessions
The fuzzy finder constructs a list of candidate sessions from the following
sources, in the following order:

- Existing tmux sessions.
- Repositories and workspaces found under `repository.globs`, in
  alphabetical order.

When reconciling existing sessions with candidate sessions, a name is generated
for each candidate session. If it matches the name of an existing session, the
candidate is discarded (existing sessions take precedence).

### Session Names and Metadata
The switcher represents sessions by their name and metadata.

- For a session without an associate repository, the name is simply the
  supplied session name, and there is no extra metadata.
- For a session with an associated repository or workspace, the name is the
  name of the directory containing the workspace root for that
  repository/workspace (derived from `workspace.template`).

### Preview Pane
The preview pane shows a live preview of the selected session, in a similar
style to tmux's `C-b s` session switcher, assuming the session already exists.

### Picking a Session
When picking a session from the fuzzy finder, all its parts are ensured to exist:

- If there is a workspace, it is set-up -- it is the CWD for the session.
- The session is created and added to tmux.

Then the pop-over switches to the session and closes itself.

### Actions
- `C-r` opens a sub-fuzzy-finder to select a different repository,
  populated by enumerating valid `jj` repositories found by evaluating
  `repo.globs`.
- `C-b` opens a sub-fuzzy-finder to select a different base revision. This
  will only be enabled if a repository is selected. The fuzzy finder is
  populated with bookmarks (including `*@origin`), with `trunk()` included as
  a pseudo-entry at the top.
- `C-n` will create a new session from the current query. This will first
  check that a session with this name doesn't already exist, and if so,
  follows the "picking a session" flow above.
- `C-x` will close a given (existing) session. This involves closing the
  session in tmux.
- `C-d` will delete an existing session and/or workspace. If there is a
  session for this selection, it is closed in tmux. If the selection is tied
  to a real repository/workspace candidate, the workspace is forgotten in `jj`
  and deleted from disk; otherwise it behaves like `C-x`.

## Tech recommendations
This tool will be built using Rust, taking advantage of `skim` for fuzzy
finding, and shelling out to `jj` and `tmux` for everything else.
