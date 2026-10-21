# B638: holes in the split view's bottom half

## Survey
Every hide/swap keyed to the main camera's eye, checked against the split (450×900 doubles, `HST_WEATHER=0`,
`--play --stage N --shot-at 16`, stages 1–10, culling on vs `--cull off`; `context/b638/run2/`). Note: `tools/shot.sh`
runs whatever `target/release/hst` is there, it doesn't build: `cargo build --release -p hst` first.

- Bevy's frustum cull (`cull.rs`) and the directional light's cascades: per view already, fine in every bottom half.
- Shadows (`shadow.rs`): `reaches_court` is symmetric about the net, so nothing past either baseline drops out;
  projected shadows aren't eye-keyed.
- Stage (`stage.rs`): nothing hidden by the eye there.
- Court mod people (`npcs/court_people.rs`): one entity per person, so the bottom half saw the top's clip and turn.
  I wrote twins on per-half layers, but B639 landed the same fix (`twin_view::TopHalf`/`BOTTOM`) first, so mine
  was dropped and the branch restarted from main.
- Holes found before B634 landed: stage 9's bridge girders (net_hide's `NearBox`) and stage 5's end stand behind
  the top team (`cull_crowd`) missing in the bottom half. B634 now runs both culls on either half's view; on
  main's build both show in the bottom half (`context/b638/run6/c5.png`, stage 9 likewise).

## Also changed (user, mid-task)
- "Still not the singles personal camera, they're so zoomed out": `split_view::place` widened each half's fov so
  the game's whole 4:3 width fit across a near-square half, adding height and shrinking everyone. Each half now
  shows the 4:3 picture's height (as the one-human camera does on a wide screen); the sides crop.
- "Landscape: normal camera by default, 1P toggles the split": the personal split's choice is no longer kept in
  `split.txt`; every match starts on the shared camera (`HST_SPLIT2=on` still starts split for shots). Shots:
  `run6/l5.png` (landscape 2P singles: normal), `w5.png` (toggled: side by side, close), `s5.png`/`c5.png` portrait.

## Not verified / not 1:1
- Court mod people in the split not shot (no court mod with `npcs.json` exists); B639's test covers them.
- A half narrower than 4:3 crops the picture's sides: a player serving wide sits at the edge of their half
  (`w5.png`, 2P). Remaster-only presentation, judged by eye at one moment per court.
- A user's existing `split.txt` is now ignored (left on disk).
