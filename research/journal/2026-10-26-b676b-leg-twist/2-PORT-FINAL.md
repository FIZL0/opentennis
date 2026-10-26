# B676b: thigh twist blend, port phase

Ported from `SPEC.md` with the decompile locked.

## What landed
- `hst_sim::pose::thigh_twist(character, costume)`: the spec's factor table (0.5 / 1.0, costumes 0–4 vs 5–9).
- `hst_sim::pose::untwist_thigh(local, rest, s)`: the rule, on the game's row-vector matrices, in host arithmetic
  (looks only). The degenerate case (animated axis exactly on the rest axis) returns the animated local.
- `CharacterData::thigh_twist`: the disc body's factor from the table (`load_disc_with`, so disc-body mods keep it);
  1.0 for mod bodies, NPCs and umpire mods (their thigh skin isn't the original's).
- `character::animate` → `untwist_thighs`: at the end of the drawn pose (after the crossfade, arm IK, a mod's feet and
  grip, the serve's ready blend and the ease), each thigh joint takes the new rotation and its children (the calf)
  the opposite turn in their locals, so the calf, feet and toes keep their animated world placement. It's applied
  only to the transforms written out, not to the kept pose, so an ease never starts from an untwisted thigh. The
  front-end previews run through the same `animate`, so they get it too.

## How it was checked
- `crates/hst-sim/tests/drawn_pose.rs` `thighs_untwist_like_the_game`: B676's node dumps (`fixtures/b676/s4.bin`,
  `s5.bin`: characters 6 (costume 9), 4, 3, 11, 0, 2, 1, 5). For every sampled thigh: the rule on the dump's sampled
  local times the pelvis's drawn world against the game's drawn thigh, within 2e-5 per rotation entry (the real
  misses are ≤ 4e-6); with factor 1.0, the plain local. Results: 2185 untwisted thighs, 2202 plain. Skipped: crossfade
  frames, where the dump's sampled rotation isn't the local the world came from (the calf doesn't hang off it
  either), and one frame (s4 v8204, the frame the serve stance starts) where the game drew both players' thighs
  plain (4 thighs).
- `character.rs` `thighs_untwist_and_leave_the_calves`: character 0's serve stance on our Bevy side: the thigh's
  bone axis is kept, its roll changes by more than a degree, and thigh·calf is unchanged; character 1 is untouched.

## Not verified / not 1:1
- One frame where the game skips the blend (s4 v8204, as the serve stance starts) isn't reproduced: we always
  blend. It's one frame of skin twist on a cut, which the spec doesn't describe; not worth a spec round.
- Characters 7–13 and Carol in costumes 0–4: their factor comes from the spec's table; no dump has them.
- No side-by-side screenshot against PCSX2: the math is pinned against the game's own matrices, and the effect is a
  skin roll of a few to ~27 degrees on the thighs only.
- The drawn rotation goes through Bevy's quaternion (float rounding ~1e-6), not bit-exact; this is rendering.
