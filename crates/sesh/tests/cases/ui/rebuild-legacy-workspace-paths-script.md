# Rebuild legacy workspace paths script

Legacy jj repositories can have a missing workspace path index even after named
workspaces already exist. The recovery script should rebuild that index, and
`sesh` should then use the recorded default workspace as the base when creating
a new workspace from a named workspace checkout.

    :bins jj tmux sh sed python3

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :copy ../../scripts/fix-jj-workspace-index.py fix-jj-workspace-index.py

    :t rename-session -t 0 runner
    :$ jj git init beta
    :$ jj workspace add -R beta --name zeta beta.zeta
    :$ python3 -c 'from pathlib import Path; Path("beta/.jj/repo/workspace_store/index").unlink()'
    :$ sh -c 'python3 fix-jj-workspace-index.py --repo beta default="$PWD/beta" zeta="$PWD/beta.zeta" >/dev/null'
    :$ sh -c 'jj workspace list -R beta --no-pager --color never --template "name ++ \"\\t\" ++ root ++ \"\\n\"" | sed "s#$PWD/##g"'

    :t new-session -d -s ui "cd beta.zeta && sesh"
    :t resize-window -t ui:0 -x 120 -y 10
    :pane ui:0.0
    :settle -d 2s

Type a new workspace name. Because the script restored workspace roots, the new
workspace should be based on the recorded default workspace instead of the named
checkout path.

    :k omega
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

    :k Enter
    :settle -d 2s

Accepting the row should create `beta.omega` and switch to the `beta/omega`
session.

    :t display-message -p '#{client_session}'

    :t list-sessions -F '#{session_name}:#{b:@sesh.repo}'

    :$ sh -c 'jj workspace list -R beta --no-pager --color never --template "name ++ \"\\t\" ++ root ++ \"\\n\"" | sed "s#$PWD/##g"'

---
vim: set ft=markdown:
