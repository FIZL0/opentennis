# M146: 2P singles on one device: a personal split view, each player's own camera

## What
`crates/hst-play/src/play/split_view.rs` (the B633 split, extended):
- **Eligible**: singles, both players human on this device, not online, the remaster's look (not `original`),
  on Android/iOS. `HST_SPLIT2=1` allows it on any machine; `HST_SPLIT2=on` also switches it on at startup (for shots).
- **Switch**: only 1P's Select (`Action::Camera`, their keys or pad) toggles it, only in live play (Serve/Rally, no
  cut-away, no replay), like the alternate camera. 2P's Select does nothing (user decision). The choice is kept
  in `split.txt` beside `camera.txt`.
- **Cameras**: while on, each player has their own `hst_sim::camera::Camera` (the one-human match camera,
  `turned` = that player's end), stepped each tick on the tick's starting scene, after `control` and before
  `simulate`, as `own_camera::step` does. It cuts on the shared camera's cut (new point), when the end changes, and
  when it is first switched on. Shown only when the plain match view would be (B633's conditions minus the
  tall-window check). At other times (intro, cut-aways, calls, replays, change of ends) the full shared picture shows.
- **Layout**: a wide window puts the halves side by side (1P left), a tall one stacks them (1P on top). Each half's
  fov uses the match camera's rule, never less than 4:3 across, at the half's aspect. It takes priority over
  B633's automatic team split. The shared tail (second camera, viewports) is now `show()`, used by both.
- **Sticks**: the `StickTurn` resource (per seat) is set each tick. A seat turns when its player's own picture looks
  from a different end than the one its stick follows in play (`own_camera::flipped`, the shared camera's). In
  `tick_input::local`, `turn_sticks` mirrors that seat's stick bytes and swaps its d-pad up/down and left/right
  *before* the tick's pads are pushed. Replays, the replay recorder and the tick hash therefore see what was
  played, and nothing in the sim changes. Online never gets here (not eligible).

## Checked
- `split_view::tests::each_human_has_their_own_camera` (disc): 2P singles with the split on vs without, 300 ticks.
  The match state (shared camera, ball, players, the tick's pads) is identical. Each player's camera eye stays
  behind them, and exactly one seat's stick turns: the one whose own end isn't the shared camera's.
- `a_turned_stick_runs_the_other_way`: over the whole byte grid, `pad_dir` of the turned stick is the negated run
  (within 0.02, the 128/127 byte rounding). The d-pad swaps, and other seats are left alone.
- Shots (`tools/shot.sh`, `context/m146/run1/`): 1280×720 `wide.png`/`wide1.png` show 1P left and 2P right, each
  from behind, with each score panel in its own player's half. 450×900 `tall.png` shows 1P on top and 2P below.
  The camera, own-camera and tick-input tests still pass.

## Not verified / not 1:1
- A remaster-only feature with no original counterpart, judged by eye only. No phone, tablet or touch-pad run:
  checked on desktop with `HST_SPLIT2`.
- Not driven live with two real pads: the Select toggle and the turned sticks are covered only by the tests (the
  toggle path is the alternate camera's, which is tested).
- In this slot's release build the court models didn't draw in any `--shot` (also with the split off, courts 1
  and 3). Players, crowd and HUD drew. This is environmental, not from this change. The shots show only the
  players' placement.
- Billboards (name tags, markers) face the first half's camera only, so they appear mirrored in the second half.
  The around-eye hiding and lens flare follow the first half only. Same as B633 (→ B634).
- Side by side, the score panels stay where they are (top-right 2P, bottom-left 1P), which happens to put each
  in its player's half. They aren't laid out per half.
- Positional sound is still heard from the shared camera's end.
