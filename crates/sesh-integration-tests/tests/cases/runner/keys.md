# Runner keys directive behavior

## Sends literal text and named keys to active pane

The runner should send text keys literally, then named keys like `enter`, to the current active
pane.

:bins cat
:tmux new-window -d -n keys-a "cat > keys-a.txt"
:pane 0:keys-a.0
:keys "hello" space "world" enter C-d
:sh cat keys-a.txt

## Pane switching sends keys to selected pane

Switching active pane should route keys into the newly selected pane.

:tmux new-window -d -n keys-b "cat > keys-b.txt"
:pane 0:keys-b.0
:keys "pane-b" enter C-d
:sh cat keys-b.txt

## Sends hello world as one literal text key

The runner should send a whole phrase as one literal text payload when quoted as a single token.

:tmux new-window -d -n keys-literal "cat > keys-literal.txt"
:pane 0:keys-literal.0
:keys "hello world" enter C-d
:sh cat keys-literal.txt

## Sends punctuation and capitals as literal text

Literal text keys should preserve case and punctuation.

:tmux new-window -d -n keys-punct "cat > keys-punct.txt"
:pane 0:keys-punct.0
:keys "Hello, world!" enter C-d
:sh cat keys-punct.txt

## Sends complex modifier combinations

Complex modifier combinations should be forwarded to tmux as key codes, including explicit
`btab`.

:tmux new-window -d -n keys-mod "cat -v > keys-mod.txt"
:pane 0:keys-mod.0
:keys C-a M-a C-M-a btab C-btab S-up enter C-d
:sh cat keys-mod.txt

## Bare modifier names are treated as text

Bare modifier key names should be treated as plain text tokens.

:tmux new-window -d -n keys-modtext "cat > keys-modtext.txt"
:pane 0:keys-modtext.0
:keys ctrl space opt space shift enter C-d
:sh cat keys-modtext.txt

## Failed pane selection does not retarget active pane

If `:pane` fails, key input should still go to the last successfully selected pane.

:tmux new-window -d -n keys-c "cat > keys-c.txt"
:pane 0:keys-c.0
:pane does-not-exist
:keys "still-c" enter C-d
:sh cat keys-c.txt

## Invalid active pane reports send-keys error

Sending keys to a removed pane should emit a warning.

:tmux new-window -d -n keys-d "cat > keys-d.txt"
:pane 0:keys-d.0
:tmux kill-pane -t 0:keys-d.0
:keys "boom"

---
vim: set ft=markdown:
