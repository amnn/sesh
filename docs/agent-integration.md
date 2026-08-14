# Agent integration

Agent harnesses can publish pane-scoped lifecycle state to `smth`. The picker
shows that state on live sessions, calls attention to settled or blocked agents,
and switches directly to a window that needs attention.

## Publishing lifecycle state

Publish transitions from the tmux pane where the harness is running:

```sh
smth agent idle
smth agent running
smth agent waiting
smth agent succeeded \
  --title "Implement notifications" \
  --summary "Implemented the requested change"
smth agent failed
smth agent exit
```

Harnesses can publish `idle`, `running`, `waiting`, `succeeded`, or `failed`.
These states represent a ready harness, an active run, a run waiting for user
input, and the two terminal outcomes of a settled run. `exit` stops tracking the
agent and removes its state.

`--title TEXT` and `--summary TEXT` supply one-shot notification text for that
transition. Neither value is persisted in tmux metadata. Publishing `running`
can also clear any pending notification associated with the pane. See
[Notifications][note] to configure delivery and clearing.

`smth agent` writes the state to the `@smth.agent.state` user option on the
invoking pane, selected through `$TMUX_PANE`. The value remains until the next
update or `exit`. Inspect it with:

```sh
tmux show-options -pqv -t "$TMUX_PANE" @smth.agent.state
```

[note]: notifications.md

## Picker indicators and attention

The session list summarizes agent state at the right edge of every live session
row. Its header aggregates the same summary across all live sessions,
regardless of the active filter.

| State     | Indicator | Needs attention |
| --------- | --------- | --------------- |
| Waiting   | `⏸`      | Yes             |
| Failed    | `×`       | Yes             |
| Succeeded | `✔`      | Yes             |
| Running   | `▶`      | No              |
| Idle      | `○`       | No              |

An indicator appears by itself for one agent, or with a count for multiple
agents in that state. Indicators and counts remain undimmed while dim middle
dots separate states, for example `⏸ · × · ✔ 2 · ▶ 3 · ○`. A session may
contain multiple tracked harnesses as long as each runs in its own pane.

![Agent panes transitioning through lifecycle states][demo]

Waiting, failed, and succeeded agents activate the same attention pip as a tmux
bell. Succeeded agents remain attention-worthy because they should receive
another prompt or exit. When switching to a session, `smth` selects the first
window with either a bell or agent attention before falling back to the
session's ordinary target.

[demo]: assets/agent-attention.gif

## Pi extension

This repository is also a [Pi package][pkg].
After installing the `smth` binary, install its Pi lifecycle extension directly
from this repository:

```sh
pi install git:github.com/amnn/smth
```

The Pi package is isolated under `extensions/pi-smth` and includes both the
lifecycle extension and an on-demand `smth` session-control skill. For local
development, load that workspace package without installing it:

```sh
pi -e ./extensions/pi-smth
```

The extension activates when Pi is running inside tmux. It publishes `idle`
when the Pi session starts, `running` when an agent run starts, `succeeded` or
`failed` once the run fully settles after automatic retries, compaction, and
queued follow-ups, and `exit` during session shutdown. Settled updates include
a bounded summary of the final assistant text.

Notification titles use the current Pi session name as `pi · TITLE`, or just
`pi` when the session is unnamed. Session naming is left to Pi or dedicated
session-naming extensions.

Pi does not expose a generic lifecycle event for arbitrary prompts that block
on user input, so the extension does not infer `waiting` state.

The bundled skill activates when Pi is asked to inspect, create, switch, close,
delete, flag, or unflag `smth` sessions. It starts with `smth --json`, carries
the returned `base` and `session` identity into a strict lifecycle command, and
avoids direct tmux or jj workspace mutations. It distinguishes close from
delete and requires confirmation before deletion unless the user already made
an explicit deletion request. See [Scripting][script] for the complete CLI
contract.

[pkg]: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/packages.md
[script]: scripting.md

### Development

Validate the extension package from the repository root with:

```sh
pnpm install
pnpm check
```

The full check verifies formatting, type-checks the package, runs its tests,
smoke-tests extension loading without a model request, and audits dependencies.
