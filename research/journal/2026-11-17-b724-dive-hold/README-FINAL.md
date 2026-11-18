# B724: the contact dive's hold phase

B721 found that on the weight-0 ticks between the dive arm solve's ramp-up and ramp-down, the game's pose keeps the
full turn. Play drew the arm unturned and the racket stock there.

## Checked on the recordings

This uses `fixtures/b721_dive_c00.bin{,.nodes}`: P1 (Ashley) with three dives, two misses (contact frames 11 and 6)
and one contact dive (frame 15). The node dump was recorded with
`research/journal/2026-11-17-b721-dive-arm-check/scripts/b721_dive_nodes.py`.

- **Every dive pauses:** contact and miss dives alike leave each node's local rows and held rotation bit for bit as
  on the tick before, on dive ticks 6 to the contact frame. Those are exactly the ticks `DiveIk` weighs 0 after its
  ramp-up. No other dive tick is frozen. The world rows still move: the placement follows the slide.
- **Contact dive locals:** on its hold ticks, the right upper arm's local is its animated rows times the full solved
  turn, and the Racket local is the full racket turn (weight 1). Both match bit for bit.
- **Tests** in `crates/hst-sim/tests/dive_ik.rs`:
  - `dive_hold_freezes_the_pose` checks the freeze on all three dives;
  - `dive_solve_as_recorded` now checks the hold ticks at weight 1, as well as the ramp ticks.

## Port

- **Body:** play already holds the receive motion's clock (speed 0) on dive ticks 4 to frame − 2. With play's
  posed nodes two ticks behind the original's, those are the game's ticks 6 to the contact frame. So the body pause
  was already in place.
- **Arm and racket:** `dive_ik::tick` now shows weight 1 on the hold ticks (weight 0 after the ramp-up). The drawn
  arm and racket keep the full turn, and so does the next doubles player's stand-test read (`doubles_ai::scratch`
  reads `Player::arm` / `Player::racket`; it needed no change).
- **Unit test:** `dive_hold_keeps_the_full_turn` covers the weight sequence.

## Not verified / not 1:1

- **One contact dive only:** Ashley, right-handed, contact frame 15. Not covered by a recording:
  - a left-handed diver;
  - a doubles dive;
  - a dive whose slide a blocked move cut short. Play's motion restarts at time 5 on that one; the hold rule here
    still follows the IK counter only.
- **No frame-by-frame check in play:** I didn't compare play's drawn dive against the game frame by frame, and
  didn't record a doubles stand test that reads a racket during a dive's hold. The rule is proven on the game's
  nodes and in a play unit test only.
- **Placement during the hold:** the game's frozen pose sits on a placement that keeps sliding. Play matches this
  by construction (it holds the motion clock), but I didn't compare it on screen.
