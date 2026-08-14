# Delete workspaces

`--delete` resolves a discovered named-workspace session, forgets and removes
its checkout, then closes it when live.

    :bins jj tmux cat test

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init alpha
    :$ jj workspace add -R alpha --name feature alpha.feature
    :$ jj workspace add -R alpha --name other alpha.other

Create a plain collision and a disambiguated repo-backed feature session. Strict
metadata should ensure deletion closes only the latter.

    :t new-session -d -s alpha/feature "cat"
    :t new-session -d -s alpha/feature~1 -c alpha.feature "cat"
    :t set-option -F -t '=alpha/feature~1:' @smth.repo '#{pane_current_path}'
    :t new-session -d -s scratch "cat"

    :$ smth --base alpha --delete feature
    :t has-session -t '=alpha/feature~1'

    :t has-session -t '=alpha/feature'
    :$ test ! -e alpha.feature

A discovered workspace without a live tmux session should still be forgotten
and removed through the same session operation.

    :$ smth --base alpha --repo "alpha*" --delete other
    :$ test ! -e alpha.other

Only the default workspace should remain registered.

    :$ jj workspace list -R alpha --ignore-working-copy --no-pager --color never --template 'name ++ "\n"'

Missing sessions and the default checkout are not deletable.

    :$ smth --base alpha --delete missing

    :$ smth --base alpha --repo "alpha*" --delete default

    :$ test -d alpha

Delete rejects the plain-session namespace and never closes a same-named plain
session on an invalid request.

    :$ smth --no-base --delete scratch

    :t has-session -t '=scratch'

The operand is mandatory, picker controls are rejected, and delete is mutually
exclusive with every other lifecycle action.

    :$ smth --base alpha --delete

    :$ smth --base alpha --delete missing --query missing

    :$ smth --base alpha --delete missing --close missing

---
vim: set ft=markdown:
