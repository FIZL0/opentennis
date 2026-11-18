# B721: the contact dive's arm solve against the game

Recorded Ashley (character 0) as P1 in a singles match diving for balls on purpose, with the diver's node array
on every dive frame, the shot path record, and P1's whole object at the contact dive's start
(`scripts/b721_dive_nodes.py <char> <out.bin> [frames] --slot 8 --dump [--all]`). The fixture is
`fixtures/b721_dive_c00.bin{,.nodes,.path,.dump}` (context/b721/run5, one right-handed contact dive, contact frame
15). The test is `dive_solve_as_recorded` in `crates/hst-sim/tests/dive_ik.rs`.

## Result

`pose::dive_solve` is now bit-exact against the game. Checked against:

- the game's own solve caught in the object at the dive's start: the upper arm's turn (quaternion) and the racket
  angle;
- the posed Racket local (rows 0–2) and the right upper arm's local on all 9 ticks with a weight above 0: 5 ramping
  up, 4 ramping down.

## Findings

- **Which way up the ball goes in:** game space, y down. The ball's height goes in negative, so the arm aims at the
  ball mirrored in the ground. With the height positive (y up, as play passed it), the turn is a different one
  entirely, not a few ulps off. Play's dive start (`hst-play/src/play/dive_ik.rs`) took the doubles AI's path window
  y up and now flips it. That changes how the dive looks, so it gets a release-notes line.
- **Which velocity:** the path's velocity at the contact frame, before the hit. That equals the live ball one frame
  before contact, flown one frame. The live ball at the contact frame already carries the return.
- **Rounding:** the solve runs on the FPU (chop) and normalises with one reciprocal of the root, then three products.
  - Every length and dot product sums the squares/products y, x, z, as `swing::arm_slide` does.
  - The ball-to-shoulder vector is (ball − path end) − shoulder.
  - The racket's direction in the world is d·row summed y, x, z, with the world matrix folded left (racket · hand ·
    forearm · turn · above · placement).
  - Before this task the dots summed z, y, x. That left the turn a few ulps off and the angle about 3 ulps off.
- **The rig:** every load-time matrix `dive_rig` builds matched the game's copy in the object bit for bit.
- **Quirk in the hold:** on the ticks between ramp-up and ramp-down (weight 0), the game's posed upper arm and Racket
  locals stay at the full turn. The upper arm's held rotation doesn't change on those frames, so the dive motion is
  paused and the nodes aren't re-posed: the last turned locals persist. They start moving again on the first
  ramp-down tick. Play instead draws the arm unturned and the racket stock for those ticks, and the doubles stand test
  reads that unturned racket. Follow-up: B724.

## Not verified / not 1:1

- **One dive only:** one character (Ashley, right-handed), one contact frame (15), a small upper-arm turn (about 3.6°) and a racket angle of about −55°.
  These were not exercised:
  - a left-handed diver (the hand mirror);
  - the racket angle ≥ π/2 branch;
  - doubles.
  The op order is proven on this dive only. A larger turn could still expose a branch this dive doesn't reach.
- **The hold-phase frozen turn** (see Quirk above) isn't ported: play shows no turn between ramp-up and ramp-down.
  That's a visible difference, and in doubles possibly a different stand-test racket read. Follow-up: B724.
- **Play's sign fix** is checked only through the hst-sim solve and an hst-play build plus its dive tests. I didn't
  compare a play-side dive frame by frame against the game.
