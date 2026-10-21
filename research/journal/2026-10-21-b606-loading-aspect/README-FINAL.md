# B606 — loading screen at any aspect

From the repo audit (`2026-10-21-repo-audit`): the loading screen's 640×448 screen was a box at 100% height with a
4:3 aspect and `max_width: 100%`, which can squash in a portrait window.

## Change (`crates/hst/src/loading.rs`)
- The 640×448 screen (pane, logo panel, hint, "Now Loading...") is now sized from both window sides: width
  `min(100vw, 133.3vh)` and height `min(75vw, 100vh)` through the width/height + max clamps, so it is always the
  largest centred 4:3 box and scales uniformly. It isn't pinned with `widescreen::screen_43` (the HUD's `fit`
  system): the cover covers the whole window, not the Aspect Ratio setting's view, and `fit` works in the view's
  aspect, so a fixed Aspect Ratio on a wider window would have stretched it.
- The court picture also covered only the window's width (bars above and below in portrait), and inside even a 4:3
  window it kept its BMP's own 640:448 shape (Bevy's default image mode), leaving black bars top and bottom (~3% each).
  It now uses the court-mod picture's `covering` node at 4:3 and draws stretched over it, so it covers the window at
  any aspect (cropped at the sides when taller, top and bottom when wider), the way the original shows it at 4:3.

## Check
Temporary (not committed) `HST_B606` Startup hook in `main.rs` drawing `loading::cover` for court 6 under `--shot`.
`context/b606/run4/`: `wide` 1280×720, `tall` 600×1000, `phone` 400×1000, `ultra` 1800×600, `four3` 800×600:
no black rows or columns in any, the pane centred and in proportion (phone: pane 538×340 px on a 640-wide shot, as
the 4:3 screen 640×480 predicts). `run1/`–`run3/` are the before/intermediate shots (bars at 4:3 and portrait).

## Not verified / not 1:1
- No original to match at portrait/ultrawide: the original is 4:3 only; the layout outside 4:3 is ours (front-end,
  PLAN: not 1:1).
- Checked through a temporary `--shot` hook, not a real menu → match load (the cover is up for a short time only).
