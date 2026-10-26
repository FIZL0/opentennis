# M148 — bigger 3D preview on the character select

## What changed
- `roster.rs`: the section rail (« Hot Shots Tennis 16 | Hot Shots Golf Fore! 24 ») is gone; its 24 px + row gap
  went to the top row (`TOP_H`), so the grid keeps its exact height. The thumbnail grows with the row, keeping its
  124:172 shape (still capped at 40 % of the column so the words beside it fit). The grade tiles beside it get the
  extra height too (bigger letters).
- B425's Get more chip (no mods installed, 1P only) moved from the rail's end to the right end of the grid's first
  heading on view; its test follows (`tabs.rs`). The rail's code (`rail`, `rail_offset`, `rail_widths`, `clip_x`) is
  removed: nothing else used it.
- The thumbnail's preview quad carries `previews::FIT`: the card frames the model to fill its box instead of showing
  the whole 4:5 picture. `inspect::drawn_share` finds what a baked picture drew (alpha > 0), kept per key in
  `Loader::drawn` (from the PNG on decode, or the bake's read-back); `inspect::frame_drawn` picks the part of the
  picture that fills the box's shape with a 3 % (of the model's height) margin, never cutting the model: where that
  would run past the picture's edge, the picture shrinks inside the box instead. The PNG shows that part
  (`ImageNode::rect`); the live model draws the same part of the original's frame (`fit_live` sets its crop via
  `inspect::sub_crop`), so the PNG → live swap doesn't move. While a live card plays a reaction (win/loss/emote,
  which can reach past the pose's box) it shows the whole picture widened to the box (`inspect::whole_part`), a
  cut like the reaction itself.
- The bake (its PNGs, face icons) is unchanged: no `RENDER` bump.

## How much bigger
Measured on the cached bakes: models' drawn boxes run from 149×342 px (narrow poses: Gloria, Lola) to 299×400
(Kaito's racket held out) of 384×480. Narrow poses fill the box's height (≈1.3× on top of the box growing ~28 %);
wide poses are limited by their width (racket tip to elbow). Shots (git-ignored): `context/m148/before-roster4.png`,
`after-roster4.png`, `after-43-r2.png` (4:3), `after-portrait.png` (2:3), `after-219-r3.png` (21:9).

## Not verified / not 1:1
- Front-end screen, ours to design (PLAN.md): no original to match.
- A live card's reaction framing (whole picture during win/loss/emotes) wasn't shot: `--shot` runs read no pads, so
  no stick flick; it falls back to the same framing the card had before M148, which showed reactions whole.
- A key with no baked PNG yet (first launch) frames the whole picture until its bake lands, then zooms in once.
- The online compact card's thumbnail and the confirm/lobby/assign previews keep the whole 4:5 picture (the task
  named the stats screen only).

## Wrap-up
The work is done and its own tests pass (hst main_menu 272, hst-base inspect 38, clippy clean). done.sh's full
suite failed twice on one test outside this change, `hst-base mods::tests::oob_hair_swings_and_settles`
("oob_pc00_jasmine: the hair never settled: 0.626"). It reads `mods/hst-mods` (→ ../opentennis-mods/out/mods),
which B663 (out-of-bounds characters' scale, running alongside) rebuilt at 22:27 just before. Nothing in M148 touches
mods, springs or that folder. Unblock: once B663 lands (or its mods are rebuilt), rerun `tools/done.sh M148`.
