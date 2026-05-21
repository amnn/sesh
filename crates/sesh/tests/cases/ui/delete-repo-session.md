# Delete repo session

`C-d` should be available for a repo-backed named workspace entry even when
there is no live tmux session to close. Confirming should forget the workspace
and delete the workspace checkout.

    :bins jj tmux cat sh test

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init beta
    :$ jj describe -R beta -m "beta commit"
    :$ jj workspace add -R beta --name feature beta.feature
    :$ jj describe -R beta.feature -m "feature commit"
    :t new-session -d -s ui "sesh -r 'beta*'; cat"
    :t resize-window -t ui:0 -x 120 -y 12
    :pane ui:0.0
    :settle -d 2s

Filter to the repo-only named workspace entry. The header should offer deletion
even though the selected row has no live tmux sigil.

    :k feature C-n
    :snap

Pressing `C-d` should mark the repo entry for deletion.

    :k C-d
    :settle
    :snap --color

Confirming should leave the picker alive, forget the workspace, and remove the
workspace checkout.

    :k C-y
    :settle -d 2s
    :snap

    :$ sh -c 'test ! -e beta.feature'
    :$ jj workspace list -R beta --ignore-working-copy --no-pager --color never --template 'name ++ "\n"'

    :t has-session -t ui

---
vim: set ft=markdown:
