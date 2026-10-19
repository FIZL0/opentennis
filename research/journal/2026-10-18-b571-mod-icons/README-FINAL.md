# B571 — store-downloaded mod characters without baked face icons

## What was wrong
The roster's face icons are cut from the baker's drawings (`previews.rs`). Three ways a downloaded mod went without one:

1. **Queue order on the select.** The background queue (built at start: every disc character's costume 0, then each
   mod's costume 0 in roster order, then the other costumes) only runs while the select is up, and only the hovered
   cards jumped it. A big mods folder (the user's has ~90 characters) meant the tiles on view waited behind every
   mod before them; leave the select and they stayed blank until the next visit.
2. **Installed since the start.** A store download joins the roster only after the restart, and nothing drew it
   before then; after the restart it waited in the queue as in 1.
3. **Cut files.** `std::fs::write` straight to the PNG: a run quit mid-write left a truncated file that `cached()`
   counted as baked, so it was never drawn again, and `icons` marked it failed for the whole run (also any face read
   while its write was still under way).

The user's own caches (`~/.local/share/hst`, the main checkout) had every installed character's face by now, so the
missing icons were the wait in 1/2, not lost files.

## Fix (`crates/hst/src/play/main_menu/previews.rs`, `mods_screen.rs`)
- On the select (and the online lobby's columns) the tiles on view jump the queue, after the hovered cards: a face is
  drawn the first time the roster shows it.
- `mods_screen::unrostered`: installed character mods the roster doesn't have yet. On the Mods and main pages the
  baker draws their costume 0 (store ones: their listing picture, B335) under `Loader::extra` indices
  (`Loader::extra_index`, shared with B229's hovered-card path). The stamp is the folder's, so after the restart the
  roster finds the drawing cached and shows the face at once.
- Drawings are written to `<name>.png.part` and renamed; a cached face that doesn't decode is deleted with its
  drawing and queued again (once a run); `icons` retries failed reads after every finished drawing.

## Checks
- Tests: `previews::tests::a_cut_drawing_is_baked_again`, `a_new_install_is_drawn_before_it_joins_the_roster`;
  `mods_screen` tests pass.
- Live (`HST_MENU=chars HST_SELECT=m70`, worktree cache missing the Open Tee 2 / Out of Bounds mods): baked order was
  the hovered Malachy, then the Open Tee 2 tiles on view, then the rest; the shot at 25 s has every tile's face.
  A face cut to 500 bytes logged "its face doesn't read; drawn again" and was re-baked within the run; no `.part`
  files left.

## Not verified / not 1:1
- The install-time bake was not run against the real store: downloading would write into the mods folder shared with
  the main checkout. Covered by the unit test (index after the roster, stable each frame) and by B229's existing
  path, which draws the same way.
- Front-end only (no original to match).
