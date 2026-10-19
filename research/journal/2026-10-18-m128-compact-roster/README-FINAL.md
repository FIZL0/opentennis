# M128 — compact roster cards

User request (2026-10-18, concept board A): shrink each player column's preview to a thumbnail so the face grid
gets the room.

## What changed (`crates/hst/src/play/main_menu/roster.rs`)
- `Columns`/`Parts`: under the header band a fixed top block `THUMB_H` high (124×172 at 1080 lines = 82.7×114.7 mockup
  px); `Parts::preview` is the thumbnail (at most 40 % of a narrow column's width), `Parts::info` the words beside
  it, `Parts::pills` their pills row. The rail and the grid follow; the grid now takes every bit of height left
  (a part row peeks under the last whole line, B426), no spare going to the preview any more.
- Tiles: as many `TILE` (now 60 px, was 84) as fit, at least `MIN_COLS` = 5 → 18 / 8 / 5 / 5 a row for 1–4
  columns at 16:9. Four players at 16:9 show 5 × 5 faces (+ the next section's heading and a peeking row).
- Beside the thumbnail: the character's name, "Right · Costume 1" (`frame::dotted`), the costume/hand pills (long
  words where they fit, else "1" / "Right", else the pair scaled to fit), the play-style bar, and `grade_tiles`:
  SRV STR VOL IMP FTW as five pink tiles with the original's letter art.
- Ready! moved from over the preview to a yellow pill at the foot's right end; the "N of M in … · K characters" line
  keeps the rest of the foot.
- The B481 join bar now crosses the whole top block. COM columns go through the same `column()`, so they get the
  compact top too; online, only this machine's columns use `column()` — remote/open seats keep `compact_card` as is.
- `previews.rs` / `frame.rs` / `online.rs` needed no change (the bake is the same 128×160, drawn smaller).
- Tests: `select_fits_the_window` checks the thumbnail / info / pills inside the top block and the new per-row
  counts; three navigation tests that relied on a 2-across grid now start their moves from a known place.

Shots (context/m128/run3/): roster1 (2 cols), roster4 (16:9, 4 cols), roster4-wide (1600×600), roster2 at 4:3;
run2: lobby (local compact, remotes unchanged), chars4 portrait (540×960).

## Not verified / not 1:1
- The concept board itself (claude.ai artifact) couldn't be opened from here: layout follows the task line's
  description; spacing/sizes are my reading of it.
- Front-end is ours to design (PLAN): no comparison against the original needed.
- Portrait with four columns (540×960) is legible but tight (pills shrink a lot); same as before the change.
