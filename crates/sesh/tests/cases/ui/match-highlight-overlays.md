# Match highlight overlays

Fuzzy match highlighting should overlay the styles already used by each part of
a session row. This case matches characters in the dim session suffix, the dim
repo parent path, and the plain repo basename; the linked SVG snapshot preserves
those combined styles.

    :bins jj cat mkdir

    :copy tests/fixtures/jjconfig.toml .jjconfig.toml

    :t rename-session -t 0 runner
    :$ mkdir code
    :$ jj git init code/blue
    :$ jj describe -R code/blue -m "blue commit"
    :t new-session -d -s tone~one "cat"
    :t set-option -t tone~one @sesh.repo code/blue
    :t new-session -d -s ui "sesh"
    :t resize-window -t ui:0 -x 100 -y 10
    :pane ui:0.0
    :settle

Hide the preview so the color snapshot focuses on the list row. The query
`onecb` crosses the suffix (`one`), compact repo parent (`c`), and basename
(`b`) spans.

    :k C-p
    :settle
    :k onecb
    :snap --color

Then, move selection to the new-session row so the matched live-session row is
not reversed, making its overlaid span styles visible in the SVG.

    :k M-up
    :snap --color

---
vim: set ft=markdown:
