# Flag management

Flat lifecycle flags should resolve live sessions by repository family and
workspace identity rather than by a guessed tmux name.

    :bins jj cat sh sed

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init alpha
    :$ jj workspace add -R alpha --name feature alpha.feature
    :$ jj workspace add -R alpha --name idle alpha.idle
    :$ jj workspace add -R alpha --name other alpha.other

Create a plain session whose name collides with the natural repo-backed name,
then attach the real feature workspace to a disambiguated tmux session. Also
create live targets for the default checkout, another named workspace, and the
plain-session namespace.

    :t new-session -d -s alpha/feature "cat"
    :t new-session -d -s alpha/feature~1 -c alpha.feature "cat"
    :t set-option -F -t '=alpha/feature~1:' @smth.repo '#{pane_current_path}'
    :t new-session -d -s alpha-live -c alpha "cat"
    :t set-option -F -t '=alpha-live:' @smth.repo '#{pane_current_path}'
    :t new-session -d -s alpha/other -c alpha.other "cat"
    :t set-option -F -t '=alpha/other:' @smth.repo '#{pane_current_path}'
    :t new-session -d -s scratch "cat"

Flagging `feature` must update only the repo-backed session, not the colliding
plain session. Repeating the command is idempotent.

    :$ smth --base alpha --flag feature
    :t show-options -qv -t '=alpha/feature:' @smth.flag
    :t show-options -qv -t '=alpha/feature~1:' @smth.flag

    :$ smth --base alpha --flag feature
    :t show-options -qv -t '=alpha/feature~1:' @smth.flag

Unflagging is also idempotent. A named-workspace base supplies the operand when
it is omitted, while an explicit operand overrides that inferred workspace.

    :$ smth --base alpha.feature --unflag
    :t show-options -qv -t '=alpha/feature~1:' @smth.flag

    :$ smth --base alpha.feature --flag other
    :t show-options -qv -t '=alpha/other:' @smth.flag

Omitting the operand for a default-workspace base targets the default checkout.
Plain sessions require the empty base namespace and an explicit name.

    :$ smth --base alpha --flag
    :t show-options -qv -t '=alpha-live:' @smth.flag

    :$ smth --no-base --flag scratch
    :t show-options -qv -t '=scratch:' @smth.flag

Missing, non-live, and unspecified targets should fail instead of opening the
picker or modifying a similarly named session.

    :$ sh -c 'set -o pipefail; smth --base alpha --flag missing 2>&1 | sed "s#$PWD#<ROOT>#g"'

    :$ sh -c 'set -o pipefail; smth --base alpha --repo "alpha*" --flag idle 2>&1 | sed "s#$PWD#<ROOT>#g"'

    :$ smth --no-base --flag

Lifecycle mode rejects picker controls and every other non-interactive action.

    :$ smth --no-base --flag scratch --query scratch

    :$ smth --no-base --unflag scratch --select-1

    :$ smth --no-base --flag scratch --exit-0

    :$ smth --no-base --flag scratch --filter

    :$ smth --no-base --flag scratch --json

    :$ smth --no-base --flag scratch --unflag scratch

---
vim: set ft=markdown:
