# M129: scenery standing in the way of the play is not drawn

Court props that stand between the camera and the play are now cut away. This covers court mods and stock
courts. A replay camera sitting inside a prop also gets a clear view.

## How

Each scenery fragment is tested in the shader (`gs.wgsl`, `GS_CLEAR`, set by `GsKey::clear`). It works per pixel, so
big concave meshes are fine: one Dubno "floor" mesh holds the whole tower, and the indoor halls are single meshes.
Hiding whole objects would have blanked them.

Which materials it applies to:
- **Stock courts:** every court model except the hole (ground) model. These are the same draws that already carry
  the guard band.
- **Court mods:** every material in `court.glb` and `gimmick.glb`. Not the sky, and not the court people (`scene()`).

Characters, umpires, crowds and effects are never cut.

Rules, in Bevy's world space (y up):
- **Near the eye:** a fragment within 1 m of the eye is dropped. This handles a replay camera inside a prop.
- **The ground stays:** fragments below 0.5 m are always kept.
- **In front of the room:** otherwise, a fragment is dropped when the ray from the eye through it goes on into the
  players' room at least 0.3 m further. The room is the doubles court plus 4 m beside it and 6 m behind each
  baseline, from 0 to 3 m up.
- **Fade margin:** a 2.25 m margin round the room fades it out with a 4×4 ordered dither (3/4, 1/2 and 1/4 of the
  pixels dropped).

The cut prop still casts its shadow; the shadow pass is a separate shader. The existing `__clear` nodes (M8f5a3)
still work as before. `HST_CLEAR=0` turns the effect off, for A/B shots.

The user's report (Dubno Tower, the rooftop court): a big pole fills the camera when a player goes off to the right.
On that court, antennas (`ante1-4`, up to 14 m tall) stand at z −37..−46, around the match camera. The helicopter's
rotor (`hane_front`, 4.2–4.6 m up) also sweeps over the right half of the court. Both are cut where they cover the
room.

## Checked

Shots are in `context/m129/` (`show/` holds the before/after pairs for the user).
- **Stock courts 1–11, resting match camera, `HST_CLEAR` 0 vs 1:** the only differences are people's animation
  timing (diff masks `stock/d4.png` and `d11.png`). Nothing in the original's view is cut.
- **Dubno match, bots only, 18 shots:** the rotor blade over the court is gone and nothing else changes.
- **Dubno, fixed eyes beside the near-right pillars and inside the helicopter (`HST_EYE`):** the pillar and the house
  roof in front of the camera are cut. The camera inside the helicopter sees the court.

Tests: `gs::tests::for_court_batch_keys` covers the stock court keys (non-hole models clear, the hole not).

## Not verified / not 1:1

- **Shown in the remaster only, not compared with the original.** The original game never culls props. This is a
  remaster-only addition, so there is nothing to compare against.
- **Players and the ball aren't tracked.** The shader has no per-frame data (a global would need a storage buffer,
  which WebGL2 lacks, or a rewrite of every material each frame). A prop hiding a player who is more than 4 m beside
  or 6 m behind the court, or a ball lobbed above 3 m, isn't cut.
- **Real replay cameras weren't caught.** A replay with its camera inside a prop wasn't triggered in a run. That case
  was checked with fixed `HST_EYE` eyes instead.
- **The cut leaves a visible edge.** Cut props show a hole shaped like the room's outline on screen, softened by the
  dithered margin. The user has to judge whether that looks right (task: show before/after, don't tick before they
  approve).
