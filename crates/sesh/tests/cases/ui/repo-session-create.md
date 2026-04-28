# Repo session create

This scenario launches `sesh cli` with a repo-backed picker entry that does not
have a live tmux session. Pressing Enter on that entry should create a detached
tmux session for the repo, attach `@sesh.repo` metadata, and switch the current
client to it.

    :bins jj tmux

    :t rename-session -t 0 runner
    :$ jj git init beta
    :$ jj describe -R beta -m "beta commit"
Launch the picker in a live tmux client and make the repo discoverable.

    :t new-session -d -s ui "sesh cli -r beta"
    :t resize-window -t ui:0 -x 120 -y 10
    :pane ui:0.0

Type a query that selects `beta`, wait for the picker to redraw, and accept it.

    :k beta
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

    :k enter
    :snap -d 2s

The created session should now be present, attached to the repo, and selected by
the control-mode client.

    :t display-message -p '#{client_session}'

    :t list-sessions -F '#{session_name}:#{@sesh.repo}'

---
vim: set ft=markdown:
