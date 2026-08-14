# Close sessions

`--close` should kill only a strictly resolved live tmux session and preserve
all jj workspace state and checkout directories.

    :bins jj tmux cat sh sed test

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init alpha
    :$ jj workspace add -R alpha --name feature alpha.feature
    :$ jj workspace add -R alpha --name other alpha.other

Create repo-backed sessions for the default and both named workspaces, plus a
plain session that collides with the natural feature tmux name.

    :t new-session -d -s alpha/feature "cat"
    :t new-session -d -s alpha/feature~1 -c alpha.feature "cat"
    :t set-option -F -t '=alpha/feature~1:' @smth.repo '#{pane_current_path}'
    :t new-session -d -s alpha/other -c alpha.other "cat"
    :t set-option -F -t '=alpha/other:' @smth.repo '#{pane_current_path}'
    :t new-session -d -s alpha-live -c alpha "cat"
    :t set-option -F -t '=alpha-live:' @smth.repo '#{pane_current_path}'
    :t new-session -d -s scratch "cat"

A named-workspace base should infer `feature` and close the disambiguated
repo-backed session while leaving the colliding plain session alive.

    :$ smth --base alpha.feature --close
    :t has-session -t '=alpha/feature~1'

    :t has-session -t '=alpha/feature'
    :$ test -d alpha.feature
    :$ sh -c 'jj workspace list -R alpha --ignore-working-copy --no-pager --color never --template "name ++ \"\\n\"" | sed -n "/feature/p"'

An explicit operand overrides the inferred workspace, and omitting the operand
for a default base closes the default checkout's live session.

    :$ smth --base alpha.feature --close other
    :t has-session -t '=alpha/other'

    :$ test -d alpha.other

    :$ smth --base alpha --close
    :t has-session -t '=alpha-live'

    :$ test -d alpha

The plain namespace remains independent: it can close both an ordinary plain
session and the plain collision without consulting repository metadata.

    :$ smth --no-base --close scratch
    :t has-session -t '=scratch'

    :$ smth --no-base --close alpha/feature
    :t has-session -t '=alpha/feature'

Closing a registered workspace with no live session fails safely, as does an
unspecified plain target.

    :$ sh -c 'set -o pipefail; smth --base alpha --repo "alpha*" --close feature 2>&1 | sed "s#$PWD#<ROOT>#g"'

    :$ smth --no-base --close

Close rejects picker controls and every other lifecycle action.

    :$ smth --no-base --close runner --query runner

    :$ smth --no-base --close runner --create runner

---
vim: set ft=markdown:
