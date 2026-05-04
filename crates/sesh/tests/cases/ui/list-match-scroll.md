# List match scroll

When a fuzzy match lands beyond the visible width of a long session row, the row
content should scroll horizontally far enough to keep the last matched character
visible.

    :bins cat jj mkdir

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ mkdir -p code/alpha-supercalifragilistic-expialidocious-alpha-supercalifragilistic-expialidocious-z
    :$ jj git init code/alpha-supercalifragilistic-expialidocious-alpha-supercalifragilistic-expialidocious-z
    :t new-session -d -s alpha "cat"
    :t set-option -t alpha @sesh.repo code/alpha-supercalifragilistic-expialidocious-alpha-supercalifragilistic-expialidocious-z
    :t new-session -d -s ui "sesh"
    :t resize-window -t ui:0 -x 80 -y 10
    :pane ui:0.0
    :settle

Hide the preview, then search for the unique trailing `z`. The matched live
session row should show the end of the long repo path rather than only its
prefix.

    :k C-p
    :settle
    :k z
    :snap --color

---
vim: set ft=markdown:
