# Switch sessions

`--switch` should share create semantics, then switch the invoking tmux client
to the strictly resolved target.

    :bins jj tmux cat sh sed

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init alpha
    :$ jj describe -R alpha -m "base commit"
    :$ jj workspace add -R alpha --name feature alpha.feature
    :$ jj workspace add -R alpha --name other alpha.other

Create a live feature session with agent attention in its second window. A
named-workspace base with no operand should infer `feature`, and switching to an
existing live target should select its first attention window.

    :t new-session -d -s feature-live -c alpha.feature "cat"
    :t set-option -F -t '=feature-live:' @smth.repo '#{pane_current_path}'
    :t new-window -d -t feature-live:1 -c alpha.feature "cat"
    :t set-option -p -t feature-live:1.0 @smth.agent.state waiting

    :t respawn-pane -k -t runner:0.0 'smth --base alpha.feature --switch; tmux wait-for -S switched-feature; cat'
    :t wait-for switched-feature
    :t display-message -p '#{client_session}:#{window_index}'

A default checkout with no live session should get one before the client
switches.

    :t switch-client -t runner
    :pane runner:0.0
    :t respawn-pane -k -t runner:0.0 'smth --base alpha --switch; tmux wait-for -S switched-default; cat'
    :t wait-for switched-default
    :t display-message -p '#{client_session}:#{window_index}'

An explicit operand overrides the workspace inferred by a named base. Existing
checkouts receive tmux sessions without creating new workspaces.

    :t switch-client -t runner
    :pane runner:0.0
    :t respawn-pane -k -t runner:0.0 'smth --base alpha.feature --switch other; tmux wait-for -S switched-other; cat'
    :t wait-for switched-other
    :t display-message -p '#{client_session}:#{window_index}'

A missing named workspace should be created at `--onto`, then switched to.

    :t switch-client -t runner
    :pane runner:0.0
    :t respawn-pane -k -t runner:0.0 'smth --base alpha --onto @ --switch fresh; tmux wait-for -S switched-fresh; cat'
    :t wait-for switched-fresh
    :t display-message -p '#{client_session}:#{window_index}'

    :t switch-client -t runner
    :pane runner:0.0
    :$ sh -c 'jj workspace list -R alpha --ignore-working-copy --no-pager --color never --template "name ++ \"\\n\"" | sed -n "/fresh/p"'

Plain targets should be created in the empty base namespace and switched to in
the same way.

    :t respawn-pane -k -t runner:0.0 'smth --no-base --switch scratch; tmux wait-for -S switched-scratch; cat'
    :t wait-for switched-scratch
    :t display-message -p '#{client_session}:#{window_index}'

    :t switch-client -t runner
    :pane runner:0.0

A plain switch without a name is invalid, picker controls are rejected, and
switch is mutually exclusive with create.

    :$ smth --no-base --switch

    :$ smth --no-base --switch scratch --query scratch

    :$ smth --no-base --switch scratch --create scratch

---
vim: set ft=markdown:
