# B593 — Mods browser footer to the top

Iteration file: this README (FINAL).

The footer's row (online note, status, Refresh / Download all words, Upload) now sits right under the grid panel's
strip (kind tabs, sort, filter), with its light rule *under* it separating it from the tiles. A narrow panel keeps
the words' own row over the pills, both above the tiles. The Texture packs list (unstacked strip) moves the footer
up with the grid.

Cursor order (Browse) is now top bar → strip → footer → tiles:
- ↓ from a kind tab / the sort or filter: the footer's first word (Upload when the page has none).
- ↓ from the search box with no sort/filter: the strip (it used to skip to the tiles).
- ↑ off the tiles' top: the footer; ↓ off the tiles' bottom: stays (it used to go to the footer).
- ↑ from the footer: the strip (remembered slot, else the open kind tab); ↓ from the footer: the remembered tile.

Tests: the mods-screen moves tests updated to the new order; `footer_sits_over_the_tiles` checks the rects at
five aspects. Shots (16:9, 600×900 portrait, Texture packs) in `context/b593/run1/`.

## Not verified / not 1:1
- none (front-end is ours to design; Upload page's own moves are unchanged since its Upload button is tap-only).
