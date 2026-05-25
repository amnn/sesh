# Repository config globs

Repository globs from config should stack with globs supplied on the command
line, so both sources can surface repo-backed picker entries.

    :bins jj

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init config-repo
    :$ jj describe -R config-repo -m "config glob commit"
    :$ jj git init cli-repo
    :$ jj describe -R cli-repo -m "cli glob commit"

    :w .config/sesh/sesh.toml

```toml
[repo]
globs = ["config-repo"]
```

Launch `sesh` with an additional CLI glob, then verify each repository can be
selected from the same picker.

    :t new-session -d -s ui "sesh -r cli-repo"
    :t resize-window -t ui:0 -x 120 -y 10
    :pane ui:0.0
    :settle

The config-supplied glob discovers `config-repo`.

    :k config
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

The CLI-supplied glob discovers `cli-repo` in the same run.

    :k C-u cli
    :snap "/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{1,2}/t" "/(?:@|○|◆)\s+([a-z]{8})/w" "/\b([0-9a-f]{8})\b/h"

---
vim: set ft=markdown:
