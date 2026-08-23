# Long-running operation spinner

Creating a session in the background should keep the picker responsive and show
delayed, animated, operation-specific progress in the header until it finishes.

    :bins jj tmux cat sleep

    :t rename-session -t 0 runner

Block the session setup script so the create operation stays in flight long
enough to observe its loading state.

    :w .config/smth/smth.toml
```toml
[tmux]
setup = '''
: > spinner-ready
tmux wait-for spinner-release
: > spinner-finished
'''
```

    :t new-session -d -s ui "smth; cat"
    :t resize-window -t ui:0 -x 120 -y 14
    :pane ui:0.0
    :settle -d 2s

Start creating a detached session, then wait until its setup script reaches the
blocking point.

    :k zeta C-n
    :$ sh -c 'until test -f spinner-ready; do :; done'
    :$ sleep 0.6

The query should be cleared when creation is dispatched. Once the display delay
has elapsed, the header's left side should be overdrawn with a spinner and dark
green, animated `creating...` label, while the remaining repo context stays
visible. Each dot frame should overwrite only its visible dots and one trailing
padding cell. Normalize both animations for the snapshot.

    :snap -d 2s "/[⠋⠙⠹⠸⠼⠴⠦⠧]/⠋" "/creating(.{4})/."

Query editing and navigation should remain available, while another create
request should be ignored until the active operation completes.

    :k omega C-n
    :snap -d 2s "/[⠋⠙⠹⠸⠼⠴⠦⠧]/⠋" "/creating(.{4})/."

Release the setup script and synchronize on its completion before inspecting the
picker again.

    :t wait-for -S spinner-release
    :$ sh -c 'until test -f spinner-finished; do :; done'
    :settle -d 2s

The progress line should be gone, the edited query should remain, and only the
original `zeta` create request should have run.

    :snap

    :t has-session -t zeta
    :t has-session -t omega

---
vim: set ft=markdown:
