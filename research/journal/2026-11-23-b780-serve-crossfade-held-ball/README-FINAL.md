# B780 — Serve set-up held ball during the stance→step crossfade

## Problem (from B778)

- In both bot-match probes (`european_rngs` and the US probe in `crates/hst-play/src/play/fifty_hz.rs` tests), the
  held ball in the server's hand differed from the recording during the serve set-up's 0x20→0x21 crossfade.
- EU: recorded ticks 2–7 (e.g. tick 2: game y −0.522, play −0.825). US: recorded ticks 2–8.
- The ball matched before and after that window.

## Cause

- Play's `held_ball` (`crates/hst-play/src/play.rs`) posed the left hand's finger (Bip01LFinger21) from the new
  motion's clip alone.
- The game places the ball on the pose its motion player has mixed mid-crossfade, not on the new clip's pose.

## Fix

- New `held_pose` in `play.rs` builds the crossfaded pose the same way `bodyhit::pose` does:
  - the new clip sampled at its time;
  - the outgoing clip mixed over the first 23 tracks by this tick's weight.
- Held-fade case (a crossfade with a hold count): the outgoing pose is frozen, and the new clip's frame 0 is mixed
  over it.
- `held_ball` uses `held_pose` while a crossfade runs.
- Gameplay change; no test-only hooks.

## Results

- `fifty_hz` tests pass.
- EU: held ball bit-exact on every recorded tick 0–113. The check now starts at tick 0 (was 8).
- US: bit-exact on ticks 1–168. The check now starts at tick 1 (was 9).
- US tick 0 is still one stance frame ahead of the recording (game [2.9440882, -0.5307655, -12.321224], play
  [2.9468277, -0.5111898, -12.317473]). That tick is before the crossfade, so it is a start-state matter.
  - Experiment: seeding the server's stance at clock 1, sampled 0 (instead of B775's captured 2/1) makes tick 0 match,
    but tick 1 then lags a frame. So the game's stance seems to advance two frames between ticks 0 and 1, or its
    tick-0 ball is placed a frame stale.
  - Follow-up: B783.

## Not verified / not 1:1

- (a) US tick 0: see B783 above.
- (b) `held_pose` builds from fresh nodes each tick. A node that neither clip keys holds its rest pose, whereas the
  game's motion player keeps whatever last keyed it. No difference was seen for the finger chain in these
  crossfades.
- (c) The held-fade branch isn't exercised by these serves. It mirrors `bodyhit::pose` but is unverified against a
  recording.
