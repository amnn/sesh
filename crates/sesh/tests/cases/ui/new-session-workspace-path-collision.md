# New session workspace path collision

When a new repo-backed session would create a workspace at an already-existing
path, the workspace name is disambiguated so the derived path is available.

    :bins jj tmux mkdir sh sed

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init beta
    :$ mkdir beta.zeta
    :$ jj describe -R beta -m "beta commit"
    :t new-session -d -s ui "sesh -r beta"
    :t resize-window -t ui:0 -x 120 -y 10
    :pane ui:0.0
    :settle -d 2s

Select the default repo, set it as the current repo context, then type the name
whose derived destination already exists.

    :k beta C-r C-u zeta
    :snap

    :k Enter
    :settle -d 2s

Accepting the row should create and switch to a workspace whose derived path is
free.

    :t display-message -p '#{client_session}'

    :t list-sessions -F '#{session_name}:#{b:@sesh.repo}'

    :$ sh -c 'jj workspace list -R beta --no-pager --color never --template "name ++ \"\\t\" ++ root ++ \"\\n\"" | sed "s#$PWD/##g"'

---
vim: set ft=markdown:
