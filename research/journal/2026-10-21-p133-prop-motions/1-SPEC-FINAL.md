# P133 — court prop motions: spec phase

Spec: `SPEC.md` (this dir). Next: the port phase implements it from the spec, the P125 dump and a new court 5
recording, with the decompile locked.

## What I found
- Props get the same motion player as characters at court load. Structures (category 19) and props (18) bind motion
  0 of `<model>.ani`, but only structures ever step. Court 5's carts are structures.
- A structure steps once per court draw pass. It doesn't step on the first pass after the court is built, while
  paused, on slow-motion in-between frames, or on a pass where its visibility test hides it. Cut-aways, the intro and
  replays all count. So the clock counts draw passes, not match ticks.
- The ball's sweep recomposes a dirty prop's node matrices before testing it. So during the update the ball sees the
  pose sampled in the previous frame's draw pass.
- Checked against P125's court 5 dump (`context/p133/court05.bin`): 516 passes since the build. The carts' clocks
  read 36 with last sampled 35, and their trees are still dirty. The wheel world matrices are one sample behind the
  locals (40.32° against 50.625°). One structure was hidden for 158 passes (clock 358).

## Not verified / not 1:1
- Not checked over frames on the running game. This phase is the spec only. The per-frame wheel recording, plus a
  pause and a replay, is left to the port phase (SPEC.md *For the port*).
- The exact sample time behind the RAM world matrices (34) is inferred from the order of the code and the
  ~10° gap. The port phase confirms it with `pose::Clip` on the dump.
- I didn't check the court 6 horses' plant category. They don't collide, so it doesn't matter for the sim.
- The remaster has to count the same draw passes as the original (intro length, replays, cut-aways) for the wheel
  phase to line up. Its presentation only has to look right (PLAN.md), so a cart's phase after a replay can differ
  from the original's unless the port takes the pass count from the original's timeline. The spec flags this for
  the port.
