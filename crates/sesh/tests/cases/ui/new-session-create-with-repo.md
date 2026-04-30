# New session create with repo

`C-n` uses the current repo context when creating a new named session, so the new
session starts in that repo and records `@sesh.repo` metadata.

    :bins jj tmux

    :t rename-session -t 0 runner
    :$ jj git init beta
    :$ jj describe -R beta -m "beta commit"
    :t new-session -d -s ui "sesh -r beta"
    :t resize-window -t ui:0 -x 120 -y 10
    :pane ui:0.0
    :settle -d 2s

Select the discovered repo, set it as the current repo context, then create a
new session named `zeta`.

    :k beta C-r C-u zeta
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"
    :k C-n
    :snap -d 2s

The client should switch to the new session, and the session should carry the
selected repo metadata.

    :t display-message -p '#{client_session}'
    :t list-sessions -F '#{session_name}:#{@sesh.repo}'

---
vim: set ft=markdown:
