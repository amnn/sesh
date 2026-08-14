# pi-smth

Pi lifecycle and session-control integration for the
[`smth`](https://github.com/amnn/smth) tmux session switcher.

The extension publishes Pi's lifecycle state to `smth agent`, including the
current Pi session name and a summary when a run settles. The bundled `smth`
skill teaches Pi to inspect structured session metadata and safely create,
switch, close, delete, flag, or unflag sessions through the non-interactive
`smth` CLI instead of issuing direct tmux or jj lifecycle commands.

The extension activates only when Pi is running inside tmux. Both resources
require the `smth` binary to be available on `PATH`.

## Installation

Install the extension and skill directly from the repository:

```sh
pi install git:github.com/amnn/smth
```

See the repository's
[agent integration documentation](https://github.com/amnn/smth/blob/main/docs/agent-integration.md)
for lifecycle behavior, configuration, and development instructions.
