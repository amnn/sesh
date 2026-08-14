# Base context

Explicit base options should seed the same repository context that is normally
inferred from the current working directory. Named workspaces retain their
identity while the picker uses the default workspace as its repository family.

    :bins jj cat

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init alpha
    :$ jj workspace add -R alpha --name feature alpha.feature

Launch the picker outside either checkout with the named workspace as its base.
The header should show the normalized default workspace and the usual default
revision.

    :t new-session -d -s ui "smth --base alpha.feature"
    :t resize-window -t ui:0 -x 100 -y 10
    :pane ui:0.0
    :settle -d 2s

    :snap

`--no-base` should suppress current-directory inference even when the picker is
started inside a repository.

    :t switch-client -t runner
    :pane runner:0.0
    :t kill-session -t ui
    :t new-session -d -s ui-none "cd alpha && smth --no-base"
    :t resize-window -t ui-none:0 -x 100 -y 10
    :pane ui-none:0.0
    :settle -d 2s

    :snap

The two base selectors are mutually exclusive, and an explicit path outside a
jj repository is rejected instead of silently falling back to the current
working directory.

    :$ smth --base alpha --no-base --filter

    :$ smth --base . --filter

A named workspace whose default registration disappears should retain the
named checkout as its base, matching current-directory inference.

    :t switch-client -t runner
    :pane runner:0.0
    :t kill-session -t ui-none
    :$ jj workspace forget -R alpha.feature --ignore-working-copy -- default
    :t new-session -d -s ui-fallback "smth --base alpha.feature; cat"
    :t resize-window -t ui-fallback:0 -x 100 -y 10
    :pane ui-fallback:0.0
    :settle -d 2s

    :snap

---
vim: set ft=markdown:
