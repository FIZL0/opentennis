# B676a — Player head watches the ball: folded into B675

B676 found that the game turns each player's head after sampling the clip (up to ~100°). B675 already has that
rule in its clean-room spec, `research/journal/2026-10-26-b675-head-look/SPEC.md`, written from the decompile and
checked against a live recording. The spec covers the head bone only, the ±50° turn and 0–30° nod windows (pinned
windows explain the ~100°), 7°/tick, the shared 15-tick contact hold, and the release/track gating. B675's port phase
is still open. Writing a second spec, or porting the same rule here, would just make two branches race on the same code.

## What I checked
I ran `extra_turns` on `fixtures/b676/s5.bin` (`B676=b676/s5.bin tools/check.sh -p hst-sim --test drawn_pose
extra_turns -- --ignored --nocapture`). The Head column agrees with B675's spec:
- During the serve wait (v7560–v7780) no head turns.
- Once the rally starts, the heads turn 5–54° over the clip on run/stand motions.
- Where whole-body numbers are large, it's the crossfade frames after a motion change, not the look.

## Where it goes
The spec says the look feeds gameplay: body-hit reads the head's x-row, and it sees the previous tick's turned head.
So the port belongs in hst-sim, bit-exact. `character.rs` animate then only draws the head local the sim wrote.
For mods, it puts the same angles on the mod's own Head joint. I added a note to B675 saying this, and pointing to
fixtures/b676 and `extra_turns` as an extra live check.

## Not verified / not 1:1
- Nothing is ported (left to B675's port phase).
- Lefty mirroring is still unrecorded: no left-handed costume in any user save state (B675's open question).
