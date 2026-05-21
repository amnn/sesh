# Toggle preview

This scenario verifies that `C-p` toggles the preview pane.

    :bins jj cat

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ jj git init alpha
    :$ jj describe -R alpha -m "alpha commit"
    :t new-session -d -s ui "sesh -r alpha"
    :t resize-window -t ui:0 -x 100 -y 12
    :pane ui:0.0

The preview should be visible initially.

    :snap --color

Pressing `C-p` should hide the preview and allow the list to use the full width.

    :k C-p
    :snap

Pressing `C-p` again should restore the preview.

    :k C-p
    :snap --color

---
vim: set ft=markdown:
