# B716 — dive arm solve: port phase

I ported this from `SPEC.md` with the decompile locked.

## What landed

- `hst_sim::pose`:
  - `DiveRig` / `dive_rig`: the load-time receive pose at time 5. It holds the locals of the racket, right hand, right
    forearm and right upper arm, the upper arm's parent chain, and `dive_arm`.
  - `dive_solve`: the solve at dive start. It returns the right upper arm's turn and the racket angle (spec steps 1–8, on
    `ps2` ops).
  - `DiveIk`: the per-tick dive ramp. Weights are n/5 up to n = 5, then 0 up to n = K, then 1 − (n − K)/5; the solve ends
    when the weight reaches ≤ 0. It also gives the arm turns (identity except the upper arm) and the turned racket local.
- `hst-play`:
  - `play/dive_ik.rs`: a contact dive starts the solve right after the dive is set up. It takes the ball and velocity
    from the path window's frame K and replaces any contact IK. Each tick, after the contact IK's block, it sets
    `Player::arm` (the same path the contact IK's arm turn takes to drawing and to the M185 scratch). It also sets the
    new `Player::racket`, the turned Racket rotation, while the weight is above 0.
  - `doubles_ai/scratch.rs`: the stand test's racket chain uses that turned Racket local, with its own translation,
    before the arm turns.
  - `motions` hands `racket` to `Motion::racket`. In `hst-base/src/character.rs` the drawn Racket joint takes it,
    interpolated between the last two ticks as the arm turns are.
- I didn't port the clears from mode sets and new contact or volley solves. A dive lasts K + 51 ticks and blocks the
  searches, so the solve's K + 5 always runs out inside the dive. A re-placement rebuilds the player, which clears it.
- `bodyhit.rs` (on the task line) is unchanged. `scratch::ahead` re-samples the Racket chain from the clip, so a turn
  written into `Posed` would never reach the stand test. The scratch reader applies it instead, as it already does for
  `Player::arm`.

## Tests

- `hst-sim/tests/dive_ik.rs` `dive_ik_counts_as_recorded`: covers every dive in the live recordings (p7e, p7b, slot 5
  matches): 2 contact dives and 61 miss dives.
  - On a contact dive, the game's arm-solve ramp (+0x3fc0) is 5. Its counter and flag follow `DiveIk::tick` frame for
    frame, and the flag drops at n = K + 5 (K = 15 in both).
  - On a miss dive, the flag never comes on.
- `dive_solve_points_the_arm_at_the_ball`: on Kaito's receive pose, with the solved turn applied, the racket point lies
  on the shoulder-to-ball line (cos = 1). So R carries b onto a, as the spec says. The angle stays within ±π/2.
- `pose` unit test `dive_ramp_holds_then_drops`: the ramp's weights for K = 9 and K = 5.

## Live capture attempt

`scripts/b716_dive_nodes.py` is the P7e dive recorder, extended to also record P1's node array (64 × 0x120) on every
dive frame. During ramp-up the Racket local's rotation is Y(angle·n/5)·Lr whatever the animation is doing, so the
recorded rows give the game's angle blind. My PCSX2 copy (3) wouldn't cooperate:
- Slot 3 sometimes never finished loading (`gm` stayed 0 for 12 s while slot 5 loaded).
- One run loaded, P1 served, then within 600 frames the game was back at the title screen.
- Two runs (9000 and 12000 frames) recorded no dive.

I stopped there (AGENT.md *Stuck*) and filed B721 for the capture.

## Not verified / not 1:1

- **The racket angle and the upper-arm quaternion are not checked against the game.** They aren't in the recorded
  player range (+0x3c00..0x4000: I searched both contact dives for them), and the live node capture failed (above).
  B721 does it.
- **Which way up the ball's height goes into the solve:** I followed the spec literally (B with height as a positive y,
  the arm slide's input). In the arm's y-down frame that aims at a point mirrored below the ground. The spec says it was
  checked, but no capture confirms it; B721 settles it. The velocity's y sign doesn't matter (only |Vy| is used).
- **The solve's ball velocity in play is the path window entry's `vel`**, the path's own value. That isn't
  cross-checked either.
- **Drawn racket turn between ticks** is a slerp from the last tick's rotation to this one's (presentation). When the
  turn starts or stops, the side without a turn slerps from or to the animated rotation.
- **The clears on a mode set, a new contact or volley solve, or a serve IK are not wired**, because they can't happen
  while the solve runs (see above). If dives ever get shorter than K + 5 ticks, they would need wiring.
- **The stand-test scratch's dived racket is untested in a doubles recording:** none of the doubles fixtures has a
  contact dive.
