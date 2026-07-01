# UI sigil config

A custom UI sigil should replace the default live tmux session marker.

    :bins jj cat

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :w .config/sesh/sesh.toml
```toml
[ui]
sigil = "*"
```

    :t rename-session -t 0 runner
    :t new-session -d -s alpha "cat"
    :t new-session -d -s ui "sesh"
    :t resize-window -t ui:0 -x 80 -y 10
    :pane ui:0.0
    :settle -d 2s

The live session row should use the configured sigil.

    :snap

---
vim: set ft=markdown:
