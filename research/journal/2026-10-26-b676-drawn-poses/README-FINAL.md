# B676 — Drawn poses vs the original, every character

User report: Carol's off hand goes through her racket's strings standing at the net. In the original she holds
the throat with the head upright.

## How
`research/journal/2026-10-26-b676-drawn-poses/scripts/b676_nodes.py <slot> <frames> <out.bin> [stride]` (run under `tools/pcsx2.sh`). Each sample dumps every
player's character, motion player and node array: the sampled local rotation/position and the world matrix. I
captured user save states 3, 4 and 5 (300 samples, every 2nd frame) into `context/b676/`. Slots 4 and 5 are copied to
`fixtures/b676/` for the test.

- Slot 3: Carol serving, partner char 4 at the net, chars 3 and 11.
- Slot 4: chars 6 (Carol, right-handed costume), 4, 3, 11 in a rally.
- Slot 5: chars 0, 2, 1, 5 (all CPU).

## Findings
- **Clip, frame and Racket joint are 1:1.** In every captured motion (stances, runs, strokes) the game's sampled
  local rotations match `Clip::sample_drawn` at the motion player's sampled time. The Racket node isn't animated:
  its local transform is the racket model's rest pose under Bip01RHand, the same as ours. Outside the 8-frame
  crossfades, the off hand sits in the racket's frame exactly where our skeleton chain puts it.
- **Carol's grip is not a bug in the draw. It comes from which stance she's in.** In the square stance (STAND, motion
  0) the stock off hand cradles the head's bottom rim (0.61 up the racket joint, ~0.09 off its axis), with the racket
  low. In the turned stance (motions 1/2) it holds the throat (0.22 up), with the racket upright. That turned stance is
  what the user saw in the original. We pick the stance with the ported rule (`hst_sim::player::stance`): in doubles,
  turned only while the other team hit last. The capture shows the game doing the same.
- Stock off hand per stance (disc clips, all 14 characters, costume 0; `stances_all` during exploration): stand and
  turned stance are 0.07–0.20 from the racket's throat–head axis for chars 0,1,2,4,5,6,8,9,10,11,12,13. Chars 3 and 7
  have a free off hand (≥0.28). The turn motion is free (~1.0). The serve stance is 0.27–0.57.
- **The game adjusts two things after sampling that we don't draw:** it turns the head toward the ball (up to ~100°),
  and it bends each thigh and calf by equal angles (7–27°, e.g. 21/27° in the serve stance). The `extra_turns`
  research test prints them. Follow-ups: B676a (head), B676b (legs). Both are spec first.

## Fix (mods)
`hold_head` in `crates/hst-base/src/character.rs` used to bend a mod's off hand only while the donor's hand was near
the head's centre. So in the turned stance (the donor holds the throat) the mod's hand stayed where its own clip left
it. The gate now measures distance from the racket above the handle (`off_racket`: the axis from the throat at 0.15
to the head centre at 0.777). It holds fully within 0.2 and fades out by 0.3, so it covers both the rim and the
throat. Unit test: `off_hand_on_the_racket`.

Re-shot two mods (`--mod`, motions 0 and 2, `context/b676/shots/`). In the turned stance both mods' off hands hold
the throat. I changed no mod files, so I bumped no versions.

## Tests
`crates/hst-sim/tests/drawn_pose.rs` `stance_poses_match_the_game`: compares 100,607 sampled rotations and 1,286
off-hand positions against fixtures/b676/s4.bin and s5.bin.

## Not verified / not 1:1
- Head turn toward the ball and the thigh/calf bend: not ported (B676a, B676b; spec first, needs the decompile).
- Characters 7, 8, 9, 10, 12, 13 weren't on court in any user save state. I checked only their disc clips' geometry,
  not live nodes. The draw path is the same for every character, so they should match like the 8 I captured.
- Lefty costumes weren't captured live. Mirroring is done by the player matrix, the same in the game and the remaster.
- The 8 crossfade frames after a motion change are skipped in the test; I didn't compare them bit for bit.
- Mod re-shoots: the square-stance shots hide the off hand behind the body at this camera angle. I judged the
  turned stance only.
