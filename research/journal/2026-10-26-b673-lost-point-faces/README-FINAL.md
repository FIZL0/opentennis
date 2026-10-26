# B673 — Faces after a lost point (2026-10-26, FINAL)

Question: does the face a player shows in the post-point reaction (lost point especially) match the original —
which face track, when it starts, how long it plays, per character?

## How the original was read
- `research/face_rec.py` (new): from a save-state load, each player's playing face index per frame. The first
  controller slot of the player's anim object holds the playing morph player and a table of the player's prebuilt
  ones, one per face index; the face index is the playing one's place in that table. Text mode prints face changes
  (`--every`: every point-over frame); with an output file it writes the anim object, morph player and face index
  per frame.
- Recording: slot 5 (the bot-only doubles match, characters 0, 2, 1, 5), 9000 frames, 10 torn frames skipped,
  none missed → `fixtures/b673/face_s05.bin` (test fixture; `context/b673/` has the text logs run1/run2).
- `research/face_shot.py` (new): `tools/screenshot.sh` shots N frames into each reaction (`context/b673/shots/`).

## Found
- The face is the motion's face on every one of 9000 × 4 player-frames: the reaction's own (`re_ball` 0x2b, `gu`
  0x2c, `di` 0x2d, `gu_set` 0x2e, `di_set` 0x2f); the soft follow-throughs keep the previous face; in a doubles team
  reaction (0x30..0x34) the losing team shows the `_2` take (+5: 0x35..0x39, both partners), the winners the base
  one. Speed and looping are the motion's (reactions: speed 1, not looping).
- Timing: the face restarts with the reaction motion (it is set on the frame after the point-over phase's skipped
  one, so up to three ticks land on that frame) and then ticks as the motion clock does, clamping at the face's own
  length (e.g. character 2's `gu` 34 frames, held from then on; team takes 75..130 frames), then the next motion's
  face takes over when the next point's setup starts. The 4-frame freeze before the reaction holds the face too.
- The port already does all of this (`Motion::set`'s face rule and face clock in `hst-base/src/character.rs`,
  `play.rs` `motions`' team-reaction +5, `react`'s motions at speed 1, not looping), and each character's face
  tracks/lengths come from its own `.MOR` (bit-exact weights and lengths: `morph_weights_ram`). No gap found, no
  code change.

## Built
- `crates/hst-sim/tests/face.rs` `face_s05_reactions`: over the recording, face index = the motion's rule above
  (team consistency: one team on `_2`, the other on base), face speed/loop = motion's, and the face clock bit-exact
  through 34985 ticks and 697 restarts (5300 reaction frames, 120 frames with both teams in team reactions), using
  the same `motion::Clock` the app ticks faces with.

## Not verified / not 1:1
- Side-by-side picture of a loser's face: the original's post-point close-up framed the winner in the shots taken
  (`context/b673/shots/p0_60.png`: character 2's `gu`), and the remaster's `--character N --motion M` viewer draws the
  figure small from behind, so no matching face close-up pair was made. What a face shows is covered instead by
  the RAM proofs (which face, its clock, and its morph weights bit-exact); the renderer's morph deltas are
  "looks right" territory (PLAN) and were checked in earlier face tasks.
- Singles: the recording is a doubles match. Singles losers use the same base `di`/`di_set` rule as doubles losers
  who draw no team reaction (covered here: 0x2d frames in the recording); no singles recording was made.
- Which team is the loser is checked by consistency, not against the point's winner in this test (the app-side
  tie to the winner is `serve_check`'s B270 test).
