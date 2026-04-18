# Runner tmux directive behavior

## Successful tmux command prints stdout

Successful tmux commands should append an exit annotation and then write stdout inside a fenced
code block.

    :tmux list-sessions -F '#S'

## Tmux commands share runner socket state

Commands run via `:tmux` should operate on the same runner-managed tmux server/socket across
multiple invocations.

    :tmux new-session -d -s socket-check
    :tmux list-sessions -F '#S'

    :tmux kill-session -t socket-check
    :tmux has-session -t socket-check

---
vim: set ft=markdown:
