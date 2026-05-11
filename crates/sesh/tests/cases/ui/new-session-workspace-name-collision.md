# New session workspace name collision

When a new repo-backed session would create a workspace whose name already
exists, the workspace name is disambiguated before rendering and creation. The
session name and workspace path are both derived from the disambiguated name.

    :bins jj tmux sh sed

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init beta
    :$ jj workspace add -R beta --name zeta beta.zeta
    :$ jj describe -R beta -m "beta commit"
    :t new-session -d -s ui "sesh -r beta"
    :t resize-window -t ui:0 -x 120 -y 10
    :pane ui:0.0
    :settle -d 2s

Select the default repo, set it as the current repo context, then type the name
of an existing sibling workspace.

    :k beta C-r C-u zeta
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

    :k Enter
    :settle -d 2s

Accepting the row should create and switch to the disambiguated workspace.

    :t display-message -p '#{client_session}'

    :t list-sessions -F '#{session_name}:#{b:@sesh.repo}'

    :$ sh -c 'jj workspace list -R beta --no-pager --color never --template "name ++ \"\\t\" ++ root ++ \"\\n\"" | sed "s#$PWD/##g"'

---
vim: set ft=markdown:
