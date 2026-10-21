# P128 — ghost-touch contact normal

Spec phase done (`SPEC.md`). Finding: the game's world sweep never reports a second ghost touch in one step: every
sweep after the first ghost touch ignores the ghost material, and the step's "repeated ghost" branch is dead (its
only unfiltered sweep sits behind a mode no caller sets). The port already ignores the ghost after the first touch,
so nothing changes. The recordings agree: 28 live-ball recordings, 22 single ghost touches, none doubled
(`research/ghost_touches.py`).

Next (port phase): add a test that checks `Flight::normal` against the recorded normal on every exact live step
(the 22 ghost frames included) and that no recorded frame has two ghost touches; see SPEC.md *For the port phase*.

## Not verified / not 1:1

- Ghost touch then a real contact in the same step: none recorded; the order (ghost ignored, real contact reported)
  is from the code only. A PINE recording of a fast ball through a bush into a wall would show it; not made.
- The world sweep's triangle test is VU0 microcode, so the oracle can't run it; the ignore list is not oracle-checked.
