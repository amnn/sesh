# New session shortcut

This scenario creates several repo-backed picker entries, with both live tmux
sessions and discoverable repos that do not have live sessions. It verifies that
`C-n` is only offered when the query is non-empty and not an exact live session
name.

    :bins jj cat

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init alpha
    :$ jj describe -R alpha -m "alpha commit"
    :$ jj git init beta
    :$ jj describe -R beta -m "beta commit"
    :$ jj git init gamma
    :$ jj describe -R gamma -m "gamma commit"
    :$ jj git init delta
    :$ jj describe -R delta -m "delta commit"

Launch live sessions for `alpha` and `gamma`, while `beta` and `delta` remain
repo-only entries discovered through the CLI globs.

    :t new-session -d -s alpha "cat"
    :t set-option -t alpha @sesh.repo alpha
    :t new-session -d -s gamma "cat"
    :t set-option -t gamma @sesh.repo gamma
    :t new-session -d -s ui "sesh -r 'alpha' -r 'beta' -r 'gamma' -r 'delta'"
    :t resize-window -t ui:0 -x 120 -y 12
    :pane ui:0.0

Initially the query is empty, so the `C-n` shortcut should not be visible.
Pressing `C-n` should also do nothing and leave the picker open in the same
state.

    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{6,8})\b/h"

    :k C-n
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{6,8})\b/h"

Typing a prefix of the live `alpha` session makes `C-n` visible, because `alp`
is not an exact live session name.

    :k alp
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{6,8})\b/h"

Completing the live session name hides `C-n`. Pressing `C-n` again should do
nothing and leave the picker open.

    :k ha
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{6,8})\b/h"

    :k C-n
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{6,8})\b/h"

Completing the name of the non-live `beta` repo makes `C-n` visible again,
because there is no exact live session named `beta`.

    :k C-u beta
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{6,8})\b/h"

---
vim: set ft=markdown:
