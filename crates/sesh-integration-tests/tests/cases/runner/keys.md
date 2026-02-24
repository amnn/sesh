# Runner keys directive behavior

## Sends literal text and named keys to active pane

The runner should send text keys literally, then named keys like `enter`, to the current active
pane.

:bins cat

:t new-window -d -n keys 'tmux wait-for -S ready-keys; cat > keys.txt; tmux wait-for -S done-keys'
:p 0:keys.0

:t wait-for ready-keys
:k "hello" space "world" enter C-d

:t wait-for done-keys
:$ cat keys.txt

## Pane switching sends keys to selected pane

Switching active pane should route keys into the newly selected pane.

:t new-window -d -n new 'tmux wait-for -S ready-new; cat > new.txt; tmux wait-for -S done-new'
:p 0:new.0

:t wait-for ready-new
:k "pane-b" enter C-d

:t wait-for done-new
:$ cat new.txt

## Sends hello world as one literal text key

The runner should send a whole phrase as one literal text payload when quoted as a single token.

:t new-window -d -n text 'tmux wait-for -S ready-text; cat > text.txt; tmux wait-for -S done-text'
:p 0:text.0

:t wait-for ready-text
:k "hello world" enter C-d

:t wait-for done-text
:$ cat text.txt

## Sends punctuation and capitals as literal text

Literal text keys should preserve case and punctuation.

:t new-window -d -n pct 'tmux wait-for -S ready-pct; cat > pct.txt; tmux wait-for -S done-pct'
:p 0:pct.0

:t wait-for ready-pct
:k "Hello, world!" enter C-d

:t wait-for done-pct
:$ cat pct.txt

## Sends complex modifier combinations

Complex modifier combinations should be forwarded to tmux as key codes, including explicit
`btab`.

:t new-window -d -n mod 'tmux wait-for -S ready-mod; cat -v > mod.txt; tmux wait-for -S done-mod'
:p 0:mod.0

:t wait-for ready-mod
:k C-a M-a C-M-a btab C-btab S-up enter C-d

:t wait-for done-mod
:$ cat mod.txt

## Failed pane selection does not retarget active pane

If `:pane` fails, key input should still go to the last successfully selected pane.

:t new-window -d -n c 'tmux wait-for -S ready-c; cat > c.txt; tmux wait-for -S done-c'
:p 0:c.0

:pane does-not-exist

:t wait-for ready-c
:k "still-c" enter C-d

:t wait-for done-c
:$ cat c.txt

## Invalid active pane reports send-keys error

Sending keys to a removed pane should emit a warning.

:t new-window -d -n d "cat > d.txt"
:p 0:d.0
:t kill-pane -t 0:d.0

:k "boom"

---
vim: set ft=markdown:
