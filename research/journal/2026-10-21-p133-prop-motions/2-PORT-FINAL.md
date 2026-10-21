# P133 — court prop motions: port phase

Ported from `SPEC.md` with the decompile locked. Checked against P125's court 5 dump and a new court 5 recording.

## What's in
- `hst_sim::court::moving(iso, n)` gives `court::world`'s collision world plus `Motions`. Each colliding structure
  (category 19) whose model has a `.ani` next to it gets a `PropMotion`. On court 5 that's the four `ballboy01` carts
  (props 3–6 in list order). It holds motion 0 of the file bound to the model's nodes (`pose::Clip`), the node locals
  (`pose::Nodes`, the disc pose until the first step) and a looping `motion::Clock` at speed 1.
- `Motions::pass(world, step, hidden)` is one court draw pass. It does nothing on the first pass after the build,
  when `step` is false (paused, slow-motion in-between frames) or for a prop `hidden` names. Otherwise each motion
  wraps its time, samples there (unless already sampled and no resample is pending), then adds the speed. A stepped
  prop's world nodes are recomposed right away (`walk`, the same local · parent walk as `nodes`), so a sweep after
  the pass sees that pass's sample. That is the same as the game's lazy recompose before a sweep.
- `court::world` is unchanged for its callers (`moving(..).0`).

## Checks (`crates/hst-sim/tests/world.rs`)
- `court_5_cart_clocks_match_ram` (P125 dump `context/p125/court05.bin`): 517 passes from the build leave every cart
  at clock 36 / sampled 35. That matches the dump, as do all 7 node locals of each cart, bit for bit (the locals sit
  0x80 into each 0x120-byte node, the clock just before the node array). The dump's world matrices are bit for bit the
  port's pose sampled at 34, one step behind the locals. That is the spec's "composed at the start of the pass from
  the previous sample". The port's world nodes after the pass are the pose at 35, which a sweep in the next update
  sees.
- `court_5_cart_wheels_follow_their_clock`: a new recording, `context/p133/run2.bin` (`research/journal/2026-10-21-p133-prop-motions/scripts/p133_wheels.py`). It
  covers 7200 frames of a doubles match on court 5 from P1's first serve, two minutes of CPU rallies and points with
  two pauses. For each record and each of the 4 carts, both wheels' world matrices are bit for bit the port's pose at
  the recorded sampled time (736 reads) or the one before (28 064 reads: read after the draw pass composed the
  previous sample). The time is always sampled + 1, except 6 reads that landed inside a step (sampled written, time
  not yet advanced), which shows the step order. The clocks advance exactly one step per vsync. Over the whole
  recording the steps add up to 4 × (vsyncs − 183), and the 183 held frames are the two pauses (129 + 54). Reads near
  a pass edge show as a 0 next to a 2 and cancel out.
- `cart_clocks_hold_on_the_first_pass_paused_and_hidden`: the gates (first pass, `step` false, hidden), the wrap at
  40, and that only the carts' nodes move.

## Getting the game onto court 5
Writing the court number (0x422f90) or the words that differ between two confirm screens doesn't change the court.
Select Court opens on the confirm screen's random court, so the script saves Select Court to scratch slot 8 once.
From there, two presses left reach disc court 5, using `plan/REFERENCE.md`'s carousel order, which I found again
before noticing it was there.

## Follow-ups
- **P134**: wire `Motions` into the match. That means per-tick state in the snapshot and rollback, one pass per
  displayed court frame with the pause, slow-motion and `net_hide` gates, and ball sweeps against the stepped nodes.
  The collision `World` sits in a `Const` today. The replay fixtures need the cart clock at their start. Until then
  the match's carts keep their disc pose, as before.
- **P135**: draw the turning wheels, and court 6's horses (rendering only).

## Not verified / not 1:1
- Slow-motion gating isn't checked against the game. The 2-minute recording had no instant replay with slow motion,
  so the "only whole slow-motion steps step" rule comes from the spec alone (`step` is the caller's input).
- The hidden-prop gate isn't checked against the game. No camera eye sat inside a cart in the recording, and the
  check that hides a prop lives in the match (P134).
- The intro, cut-away and replay frames counting as passes is in the recording only as unbroken stepping through two
  minutes of points (the clock never held outside the two pauses). Individual cut-aways weren't labelled.
- Category-18 props with motion files and court 6's horses aren't modelled (they don't collide). P135 covers drawing
  them.
- The match doesn't use any of this yet (P134), so the remaster's carts still collide in their disc pose during play.
