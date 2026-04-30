# Repo session create disambiguated

When a discovered repo's default session name is already used by an unrelated
live tmux session, selecting the repo should create and switch to a disambiguated
session name using a tilde suffix.

    :bins jj tmux cat

    :t rename-session -t 0 runner
    :$ jj git init beta
    :$ jj describe -R beta -m "beta commit"
    :t new-session -d -s beta "cat"

Launch the picker and select the inactive repo row, not the existing live
session with the same name.

    :t new-session -d -s ui "sesh -r beta"
    :t resize-window -t ui:0 -x 120 -y 10
    :pane ui:0.0
    :k beta down
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

Accepting the repo row should create `beta~2`, attach repo metadata to that
session, and switch the client there.

    :k enter
    :snap -d 2s
    :t display-message -p '#{client_session}'
    :t list-sessions -F '#{session_name}:#{@sesh.repo}'

---
vim: set ft=markdown:
