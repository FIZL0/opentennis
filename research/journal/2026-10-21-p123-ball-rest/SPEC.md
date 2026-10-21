# P123 — the live ball comes to rest: spec

Clean-room spec (PLAN.md *Clean-room ports*). It describes behaviour only. The evidence is the existing live-ball
recordings in `context/fixtures/` and one PINE read of the game's up vector (copy 4, save slot 5). `scripts/ball_stops.py`
lists every rest the recordings hold.

## Correction to the old notes

The P0b notes (and the task line) say the ball stops when it is slow "on a > 45° surface". That is backwards. The ball
comes to rest when it is slow **on a surface flatter than 45°**. On a steeper one it never comes to rest by speed. Its
contacts there are always bounces, so only the rally timeout stops it. The recordings bear this out: 161 rests, 160
on flat ground and one on a prop slope tilted about 31°.

## The ball's classification

The live ball keeps a classification of its latest contact since it was launched. It has three values:

- **none**: no contact since the launch;
- **bounce**;
- **roll**: the existing rolling rule in the contact response.

It also has a fourth value, **at rest**. Every launch resets the classification to *none*. That includes a hit, a
serve, the toss and the new-point placement. A launch also resets the call to *not landed* and the step count to 0.
The response sets *bounce* or *roll* on every contact it handles, ghost contacts included.

The ball also keeps **the latest contact normal**. This is the surface normal of the latest contact any sub-step's
sweep reported. It points out of the surface toward the ball. In game space (Y down) the flat court gives
(0, −1, 0). Three rules apply to it:

- It is written for every contact the sweep finds. That includes a repeated ghost touch whose response is skipped.
- The body-hit plane step writes it too: the plane's normal, or the stand-in normal when the sweep misses.
- A launch does **not** reset it. A ball with no contact since its launch still carries the normal from before.

## The rest check (end of each live-ball step)

It runs at the end of every step of the **live ball**: the normal per-frame step and the extra plane step after a
body hit. It does not run for the path predictor or the contact-count look-ahead. It comes after all the frame's
sub-steps and after the net push (the ±0.003 z nudge when the net was touched this step). It comes before the
step's "last move" is written and before the step count goes up.

The ball comes to rest if either of these holds:

1. **Slow on gentle ground.** All three of these must hold:
   - the classification is *none* or *roll* (not *bounce*);
   - the speed is below 0.001. The speed is the length of the velocity as this step leaves it. It is the square root
     of ((vy² + vx²) + vz²), added in that order, each product added to the running sum the way `madd` does
     (`ps2`);
   - the game's **asinf** of the normal's upward component is greater than 0.7853982 (the f32 nearest π/4). The
     upward component is the negated y in game space: 1 on the flat court. Use the game's asinf, not a cosine
     threshold. It is `ps2_float::libm::asinf` if the oracle agrees (oracle name `asinf` in `research/oracle.txt`;
     not checked yet, see Open questions). With the up vector exactly (0, 1, 0), the game's dot product reduces to
     exactly −y. Its extra ±0 terms change nothing.
2. **Rally timeout.** The match is in its rally phase, and the step count since the launch (before this step adds
   one) is greater than 1800. So the ball stops on the 1802nd step since its launch. A body-hit tick takes two steps
   and counts two. The rally phase is the phase the per-frame point-over check runs in. It ends the frame after the
   point is decided, when the post-point phase begins.

When the ball comes to rest:

- the classification becomes *at rest*;
- **the call is rewritten**: *touched the net first* becomes *out after the net*, and **every other call becomes
  out**. That includes *in*, *in after the net*, *not landed* and *out after the net*. The recordings show
  in → out, out-after-net → out and not-landed → out.

After that comes the arena step. It runs every live step, whether or not the ball came to rest. If the call is still
*touched the net first* and the ball is outside the arena, the call becomes *out after the net*. Outside the arena
is the test the point-over check already uses: |x| > 10.685 or |z| > 19.885, and never when the court turns the
arena limits off. The port has no equivalent of this step yet.

## A ball at rest

While the classification is *at rest*, every step of that ball does nothing at all:

- no slow-motion scale or acceleration, no sweep, no contacts, no sounds;
- no spin-model turn, no "last move" write, no rest or arena check;
- no step count.

Position, velocity and spin keep their last values. The velocity is **not** zeroed: the recorded rests keep a speed
of about 1e-4 to 8e-4. The ball stays frozen until its next launch.

## How the point-over check uses it

- In the *in* / *in after the net* branch, "the ball stopped" means the classification is *at rest*. Coming to rest
  also rewrites the call to out, so the rest itself ends the point **through the out branch** on the next check:
  - serve: a fault;
  - rally: the hitter loses, the same verdict as any out ball.

  A ball with no call, or touched the net first, also ends through that branch once it is at rest. Today the port
  never ends those points, because "stopped" there is only the timeout and a *not landed* / *net* call matches no
  branch.
- **Body hit on the serve.** The let condition is "**the ball touched the net first**" (call *touched the net first*),
  not "the ball stopped". The port's `Rally::check` has `b.stopped` there. The full rule, for the serve only (one
  shot):
  - the body is on the server's team → fault;
  - otherwise, the ball touched the net first → let;
  - otherwise, the point simply ends.

## What the recordings show (for the port's test)

All rests in the recordings are outside the rally phase:

- post-point rolls coming to rest;
- the ball settling in the serve and pre-serve phases, call *not landed* → out, at 45 to 116 steps since its launch.

Useful frames:

- `p7b_c03.bin` frame 700, post-point. The ball rolls to rest on a **prop slope** (material 34, normal y −0.854,
  about 31°), call in → out. This is the slope/prop case the task asks for.
- `p7e_c03.bin` frames 562 / 1717 / 3387: rolls to rest on court, in → out. Frames 2681 / 3843: a bouncing ball
  whose last step turns it into a roll and leaves it at rest, out-after-net → out.
- `p7e_c07.bin`, `match_s05.bin` (71 rests), `human_smash_s04.bin`, `new_recording.bin`: many more of both kinds.

No recording has a rest during the rally phase. Prove the rally consequences by forcing them on recorded rallies, or
with a fresh PINE recording that pokes the live ball's step count to about 1795 mid-rally. Show four outcomes:

- the timeout freeze;
- in → out and the verdict;
- *not landed* → out;
- *touched the net first* → out after the net.

Also check the replay tests stay bit-exact. Freezing the post-point ball and the serve-phase ball may move things
that read the ball later, so check those match the recordings too.

## Not verified / not 1:1

- The game's asinf against `ps2_float::libm::asinf` has not been oracle-checked. The address was found and added to
  `research/oracle.txt` as `asinf`.
- No rally-phase rest was observed. The timeout path and the call rewrite during a rally are read from the game's
  code only. The recordings confirm the rewrite outside the rally.
- The arena step (net-touched ball outside the arena → out after the net) is not in any recording I checked.
- The body-hit let condition is also read from the code. No recording of a net-cord serve hitting a player was
  looked for.

## Open questions

none
