# P123 — the live ball comes to rest: port

Ported from `SPEC.md` with the decompile locked; proved against the `frames_live` recordings and the oracle.

## What changed

- `hst-sim/src/ball.rs`: `Flight` keeps whether it has touched anything since the launch (`touched`, with `rolling`
  this is the spec's none/bounce/roll classification), `at_rest`, the latest contact `normal` and `rally` (the
  timeout is armed). Every contact a sub-step's sweep reports writes `normal`; a plane step writes its plane's normal
  or the stand-in. `step_world` and `step_plane` end with `settle`: slow (< 0.001, `madd` length) and not bouncing on
  gentle ground, or past 1800 steps in the rally phase → at rest, call → out (net-first → out after the net); then
  a net-first ball outside the arena → out after the net. A ball at rest makes both steps do nothing. The path
  predictor's `step` has none of it. `Flight::relaunch` carries the normal (and the rally flag) over a launch.
- `ball::gentle`: the game's `asinf(up) > π/4`. ps2-float's `asinf` is 1–2 ulps off the game's right at the
  threshold (oracle, 20k draws: a few dozen near 0.7071 and 0.5 differ), so the port uses the game's own threshold:
  its `asinf` first passes π/4 at 0x3f3504f5. Past ±1 the game's `asinf` returns FLT_MAX, so those count as gentle
  too. `hst-oracle/tests/libm.rs` `rest_slope` checks every input within 300k ulps of it plus 20k draws.
- `judge.rs`: `rested`, `left_arena`; the serve body-hit let is "touched the net first", not "stopped".
- `play.rs`: `flight.rally = phase == Rally` before the live step; `BallState::stopped` = `at_rest` (was the
  timeout only); the four launches (new-point placement, held ball, toss, hit) go through `relaunch`.

## Proof

- `line_calls.rs` `live_ball_comes_to_rest_as_the_game`: 9 recordings, 38360 live steps bit-exact, 61 rests (one
  on `p7b_c03`'s prop slope, ~31°), 1603 frozen steps: at rest, call, step count and last move all the game's.
  Calls seen turning: in → out, out after the net → out, net-first → out after the net, out → out.
- `rally_timeout_rests_the_ball_and_ends_the_point`: forced on `p7e_c03` rally frames with the call not landed,
  in, net-first and out after the net: 1800 steps no, 1801 out of the rally no, 1801 in the rally → at rest on the
  1802nd, frozen after; the point-over check ends it (rally: out / out after the net, hitter loses; serve: fault).
- `serve_body_hit_is_a_let_only_off_the_net`.
- hst-sim (267) and hst-play (183) suites pass: the replays stay exact.

## Not verified / not 1:1

- The ball the server carries before the toss: the game moves it by the hand's motion and it can come to rest in
  the hand (67 such rests in the recordings, call not landed → out). The port re-launches the held ball every tick
  with no velocity, so it never matches those frames; its rest only sets the flag, nothing reads it before the toss
  re-launches the ball. Not tested against the recordings (the hand's motion isn't in the ball object).
- A repeated ghost-material touch in one frame: the spec says the game writes its normal; the port's world sweep
  ignores the ghost after the first touch, so it can't. Follow-up P128.
- No rally-phase rest exists in any recording; the timeout and the in-rally consequences are forced on recorded
  frames, not seen in the game. No PINE recording with a poked step count was made.
- The arena step (net-first ball outside the arena) is only unit-tested; no recording has it.
- ps2-float's `asinf` isn't the game's to the ulp (follow-up P127: fix it there, which needs a ps2-float release).
