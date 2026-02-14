# No repo preview

> [!INFO]
> **Directive quick reference**
>
> | Directive | Description |
> | --- | --- |
> | `:s` / `:sh <cmd...>` | Run host command (Rust `Command` + shlex args) |
> | `:t` / `:tmux <args...>` | Run tmux command on test socket |
> | `:p` / `:pane <target>` | Change current tmux pane target |
> | `:k` / `:keys <tokens...>` | Send key tokens and/or quoted text |
> | `:snap [dregexdrepld ...]` | Capture current pane with optional replacements |

:t new-session -d -s session-no-repo -x 120 -y 40 "sleep 3600"
:snap /zz-sesh-ui-runner/<RUNNER>/ /[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏]/*/

vim: set ft=markdown:
