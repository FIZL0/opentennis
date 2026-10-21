# P128 — port phase: pinned with a test

No behaviour to port (SPEC.md: the world sweep never reports a second ghost touch in a step, and the port already
ignores the ghost after the first touch). Added `live_ball_contact_normal_as_the_game`
(`crates/hst-sim/tests/line_calls.rs`): it steps every live-ball frame of the five recordings that hold ghost
touches (`p7e_c07`, `p7b_c01`, `p7b_c08`, `p7b_c03`, `p7b_c13`) through `step_world` the way
`live_ball_comes_to_rest_as_the_game` does, and checks:

- on every bit-exact step, the port's latest contact normal (`Flight::normal`) is the recorded one, bit for bit;
- all 22 recorded ghost-touch steps are among the bit-exact ones;
- the game's per-step ghost-touch count is never above 1 in any recorded frame.

Result: 5454 bit-exact steps (60 others are skipped as in the rest test), 22/22 ghost-touch steps stepped, no
normal mismatches. Took ~10 s with the slot wait excluded.

## Corrected P123 wording

P123's spec says the latest contact normal "is written for every contact the sweep finds. That includes a
repeated ghost touch whose response is skipped." Correct version: the normal is written for every contact the
sweep **reports**, and the sweep never reports a second ghost touch in a step. After the first ghost touch, the rest
of that step's sweeps ignore the ghost material and report the real contact behind it, if there is one. So a
repeated-ghost normal is never recorded, by the game or by the port.

## Not verified / not 1:1

- No recording has a ghost touch followed by a real contact in the same step. The order (ghost ignored, then
  the real contact reported) comes from the spec, not from a recording. A PINE recording of a fast ball going
  through a bush into a wall would show it. I didn't make one.
- The world sweep's triangle test runs on VU0, so the oracle can't check the ignore list. The recordings back
  it up (22 single ghost touches, never two in a step).
