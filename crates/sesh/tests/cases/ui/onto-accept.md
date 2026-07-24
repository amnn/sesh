# Accept onto revision

This scenario selects a bookmarked commit from the onto picker and verifies that
accepting it returns to session mode with the semantic bookmark in the header,
without the push-status marker rendered by `jj`.

    :bins git jj tmux sh sleep grep

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj config set --user user.name "Test User"
    :$ jj config set --user user.email test@example.com
    :$ git init --bare -q origin.git
    :$ jj git init alpha
    :$ sh -c 'printf "pushed\n" > alpha/pushed.txt'
    :$ jj describe -R alpha -m "pushed commit"
    :$ jj bookmark create -R alpha -r @ base
    :$ jj git remote add -R alpha origin origin.git
    :$ jj git push -R alpha --bookmark base
    :$ jj new -R alpha
    :$ sh -c 'printf "base\n" > alpha/base.txt'
    :$ jj describe -R alpha -m "base commit"
    :$ jj bookmark set -R alpha -r @ base
    :$ jj new -R alpha
    :$ jj describe -R alpha -m "working copy"
    :t new-session -d -s ui "cd alpha && sesh -r ../alpha"
    :t resize-window -t ui:0 -x 90 -y 10
    :pane ui:0.0
    :settle -d 2s

Open the onto picker and wait for its working-copy row to load. Abandon that
commit from outside the picker so the loaded revision hint becomes stale.
Accepting the stale row should reload the current repo log without leaving onto
mode.

    :k C-o
    :settle -d 2s
    :$ sh -c 'until tmux capture-pane -p -t ui:0 | grep -q "working copy"; do sleep 0.01; done'

    :$ jj abandon -R alpha -r @
    :k enter
    :$ sh -c 'until pane=$(tmux capture-pane -p -t ui:0) && ! printf "%s\n" "$pane" | grep -q "working copy" && printf "%s\n" "$pane" | grep -q "base commit"; do sleep 0.01; done'

Move from the refreshed working-copy commit to the bookmarked base and accept
it. Although the local bookmark is ahead of its remote and renders as `base*` in
the log, the semantic revision in the header should be `base`. The session query
and selection should remain unchanged.

    :k down enter
    :snap -d 2s

Create a new workspace-backed session and verify that its parent is the selected
base commit rather than the previous `trunk()` default.

    :k feature C-n
    :$ sh -c 'until tmux has-session -t alpha/feature 2>/dev/null; do sleep 0.01; done'

    :$ jj log -R alpha.feature -r @- --ignore-working-copy --no-graph --color never --template description

---
vim: set ft=markdown:
