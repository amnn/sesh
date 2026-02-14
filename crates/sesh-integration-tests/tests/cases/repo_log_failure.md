# Repo log failure preview

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

:t new-session -d -s session-repo-error -x 120 -y 40 "sleep 3600"
:t set-option -t session-repo-error -q @sesh.repo /home/test/repo
:s python3 -c "from pathlib import Path; Path('jj.stderr').write_text('fatal: failed to load repository metadata\n'); Path('jj.exit').write_text('1')"
:snap /zz-sesh-ui-runner/<RUNNER>/ /[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏]/*/

vim: set ft=markdown:
