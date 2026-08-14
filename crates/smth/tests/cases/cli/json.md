# Structured inspection

`--json` should describe live tmux sessions and discovered repository checkout
candidates with identities suitable for strict lifecycle commands.

    :bins jj cat sh sed

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init alpha
    :$ jj workspace add -R alpha --name feature alpha.feature

Create a live session attached to the default checkout, flag it, and publish an
agent state that requires attention. The runner session remains a plain live
session, while the named workspace is a non-live candidate discovered by the
glob.

    :t new-session -d -s alpha-live -c alpha "cat"
    :t set-option -F -t '=alpha-live:' @smth.repo '#{pane_current_path}'
    :t set-option -t '=alpha-live:' @smth.flag 1
    :t set-option -p -t alpha-live:0.0 @smth.agent.state waiting

    :$ sh -c 'smth --no-base --repo "alpha*" --json | sed "s#$PWD#<ROOT>#g"'

The same fuzzy matcher used by the picker should narrow structured output, and
an unmatched query should emit an empty JSON array even with `--exit-0`.

    :$ sh -c 'smth --no-base --repo "alpha*" --query feature --json | sed "s#$PWD#<ROOT>#g"'

    :$ smth --no-base --repo "alpha*" --query zzz --json --exit-0

JSON inspection must not combine with line-oriented filtering or automatic
session selection.

    :$ smth --json --filter

    :$ smth --json --select-1

---
vim: set ft=markdown:
