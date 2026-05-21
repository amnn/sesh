# New session legacy workspace store

Legacy jj repositories can have an empty workspace store index. In that state,
`jj workspace list` succeeds but reports no recorded roots. A new repo-backed
session should use the selected workspace path itself as the base for creating a
new workspace.

    :bins jj tmux sh sed

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init beta
    :$ jj workspace add -R beta --name zeta beta.zeta
    :$ sh -c ': > beta/.jj/repo/workspace_store/index'
    :t new-session -d -s ui "cd beta.zeta && sesh"
    :t resize-window -t ui:0 -x 120 -y 10
    :pane ui:0.0
    :settle -d 2s

Type a new workspace name. The current repo has no recorded workspace root, so
its own path is used as the base.

    :k omega
    :snap

    :k Enter
    :settle -d 2s

Accepting the row should create a workspace from `beta.zeta`, not from the
unrecorded default workspace.

    :t display-message -p '#{client_session}'

    :t list-sessions -F '#{session_name}:#{b:@sesh.repo}'

    :$ sh -c 'jj workspace list -R beta.zeta --no-pager --color never --template "name ++ \"\\t\" ++ root ++ \"\\n\"" | sed "s#$PWD/##g"'

---
vim: set ft=markdown:
