# B634: split view, billboards face both halves

B633's split (tall windows): the main camera draws the top half, a second camera the bottom half. Camera-facing
quads were turned to the main camera only, so from the bottom half they showed edge-on or mirrored.

## What changed
- `hst-base/src/twin_view.rs` (new): while a second view is on (`SecondEye`, set each frame by `split_view::split`),
  every entity marked `Billboard` is drawn by the main camera only (layer 28, main camera layers [0, 28]) and gets a
  copy on layer 29 drawn by the split camera only ([0, 29]). `Billboard::TURNED` quads (name tags, markers,
  balloons, surprise pop-ups, body-hit stars/words) have their own system pose the copy for the second camera
  (`Second::pose`); `Billboard::BUILT` meshes (hit sparks, foot puffs and bits, cheers, rain streaks) get their own
  mesh asset that the system fills facing the second camera (`Second::mesh`). Copies follow the original's
  mesh, material, visibility and parent; they go when the split goes; `effects::hide_empty`'s emptied effects stay
  on no layer. Layers 30/31 were the first choice, but `hud_gamma` already uses 31 (its full-screen output camera):
  the split camera drew hud_gamma's quad as a faint ghost of the scene in both halves. Hence 28/29.
- Lens flare (`flare.rs`): a second flare pair under the split camera, built for its pose and lens; the main one
  is on layer 28 while split.
- Culls: crowd (court 5) shown if either view shows it; net/prop pieces near or in front of the eye
  (`net_hide.rs`), and court mods' clear-view pieces (`court_mod.rs`) hidden if they block either eye.
- Mode teardown drops the resources: the twin systems take them as `Option`.

## Checked
`tools/shot.sh` court 1, `HST_WEATHER=0`, `--shot-at 16`, clean HST_DATA (`context/b634/`):
- 450×900 baseline (`run2/a.png`, `tags.png`): bottom half's near tag (BOB) mirrored and oversized, far tag (ANNA)
  edge-on. With B634 (`run4/a.png`, `tags.png`): both readable and sized for the bottom camera, like the top half's.
- `run3`: the layer-31 ghost described above; gone in `run4` (`ghost.png`, autocontrast).
- 1280×720 (`run4/wide.png`): one view, tags and markers as before (no second eye → twin systems do nothing).

## Not verified / not 1:1
- Court, sky and scenery are missing from every `--play` shot in this slot, the baseline included (not caused by
  this task) → B640. So the culls' union (net/prop/clear-view pieces) wasn't seen on screen; crowd is court 5 only.
- Hit sparks, foot dust, rain, cheers, surprise pop-ups, body-hit words and the second flare weren't caught on
  screen in a split shot (short-lived or weather/court-specific); checked by build and code only.
- Not covered: the ball's ribbon and bounce puffs (`effects.rs` draw_flight/draw_bounce) and the court people's
  card billboards still face the top camera → B639.
- Hiding a piece that blocks either eye can hide it from the other half's background too (a prop near the bottom
  camera vanishes far off in the top half); judged acceptable for a split picture.
- Not tried on a phone or the web build; rendering judged by eye only (presentation, no PINE proof needed).
