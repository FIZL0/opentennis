# B637 — portrait split: each half uses the personal (singles) camera

## What changed
`crates/hst-play/src/play/split_view.rs`:
- Its own portrait framing (`frame`/`Rig`: eye low behind the team, fov fitted to its players, eased follow, touch-pad
  lift) is gone. Instead one `hst_sim::camera::Camera` per team (`TeamCams`) steps every fixed tick (after `control`,
  before `simulate`, on the scene the tick starts from, as `own_camera::step`): `turned` = that team's end (player
  `team`'s `end < 0`), cut on the shared camera's cut (new point), on its end changing, or when first made. This is
  M146's per-player stepping, moved out of `Personal` so both splits share it: in singles team = player, so M146's
  personal split reads the same cameras; in doubles each camera frames both players of its team (the sim camera's
  near team = the team at its end).
- One `place` draws both splits: the portrait split puts the team the match camera looks from on top (as B633), the
  other below; M146's personal split as before (1P left/top). Interpolated by the fixed step's overstep, fov widened
  to the half's aspect as the match camera's.
- `Personal.cams` → `Personal.showing` (the bool `split` needs); stick turning (M146/B635) unchanged.

## Checked
- `split_view::tests::each_team_has_its_own_camera` (new): doubles, all CPU, 300 ticks: each team's camera is turned
  to its end, its eye behind both its players, and the match (shared camera view, ball, players) is identical with
  and without the plugin. `each_human_has_their_own_camera` (M146) still passes on the shared cameras.
- Shots 450×900 (`context/b637/run1/`): `doubles.png`/`singles.png` vs the unchanged build's `doubles-base.png`:
  each half now the normal match camera from behind its team (top: A team from behind the near end, bottom: B team
  from behind theirs). Landscape `wide.png` unchanged in code path (no split there).

## Not verified / not 1:1
- In this slot's shots no court, sky or stadium draw at all (grey background), with and without this change
  (`wide-base.png`, `doubles-base.png` from the untouched build show the same), so the halves were judged by the
  players and NPCs only, not against the court lines. Environment, not this change; worth a look on a normal build.
- B636 (players blue in the bottom half) is still open; this task doesn't touch it. In these shots the bottom
  half's players drew dark, as B636 describes.
- The bottom half no longer lifts its players above the touch pad's face buttons (B633's `covered` fit): the
  personal camera is the original's framing, unaltered, as asked. Not tried on a real phone with the touch pad.
- Presentation only, remaster-only (no original counterpart): judged by eye; nothing compared against PCSX2.

## Also
`tools/patches.py` failed on main's own `0cd71ebf` (bevy_render `src/view/window/mod.rs` changed with no patch marker
or README row), which blocked `done.sh`: its comment now says `hst patch:` and the README's bevy_render row lists the
file. No code change.
