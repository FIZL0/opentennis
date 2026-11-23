# B773 spec: why the European racket lead changes per swing

What the doubles stand test reads as player 1's leftover racket direction, and why play's look-ahead-2 model leads
it by 0, 1 or 4 frames depending on the swing. Both builds behave the same way. The European rally just happens to
contain the swing that shows it most.

## What the game reads

- The leftover z the stand test reads on tick T is the racket direction (racket node's world x axis, z component,
  times the hand sign) of the player's pose at the game's own motion clock on tick T: the clip time the motion
  player has sampled that tick. EU point 3: on rally ticks 405–411 the game's value equals play's pose at the same
  clip time (tick 409: clip time 25 in both).
- So the AI reads this tick's pose. The game has no extra pose pass and no look-ahead. Any difference from play
  comes from play's motion clock running out of step with the game's. Both builds step motions once per game tick.
  The 50 Hz build doesn't step motions at a different rate here.

## Rule 1: a fast swing keeps its speed until frame 8

- A swing that starts with few frames left to contact plays faster: speed = 8 / frames-to-contact (speed 1 when
  frames-to-contact is below 1, and no speed-up at 8 or more).
- The game keeps that speed for exactly frames-to-contact motion steps, counting the start tick's step. The clip
  clock reaches exactly 8 on the contact tick. The speed goes back to 1 after that step, so the next step is speed 1.
  - Speed 4 (2 frames): steps of 4, 4, then 1. Clock 0 → 4 → 8 → 9.
  - Speed 8/3 (3 frames): steps of 2.667, 2.667, 2.667, then 1. Clock 0 → 2.667 → 5.333 → 8 → 9.
- In other words: the per-frame contact countdown turns the speed back to 1 only after the frame's motion step, not
  before it.
- Play currently resets the speed one step early (in the same tick, before the step). Its clock then runs 0 → 4 → 5
  (speed 4) and 0 → 2.667 → 5.333 → 6.333 (speed 8/3). For the rest of the swing it trails the game by 3 frames
  (speed 4) or 1.667 frames (speed 8/3).
- Seen in both captures, on every fast start:
  - EU: 29 speed-4 starts and the speed-8/3 starts.
  - US: 16 speed-4 and 6 speed-8/3 starts.
  - Speeds of 2 and below hold for the three steps checked, as the rule says (frames-to-contact ≥ 4).
- Examples:
  - EU rally tick 390, swing to the speed-4 forehand: game clock 0, 4, 8, 9…; play 0, 4, 5, 6….
  - US phase 3 tick 429: game 0, 2.667, 5.333, 8, 9; play 0, 2.667, 5.333, 6.333.

## Rule 2: when a full-speed swing starts

- A swing that starts at speed 1 (frames-to-contact ≥ 8) starts on the same tick as the game's other motion
  switches. Play starts some of them one tick early:
  - EU: the switches to motion 0x12 at rally tick 180 and 0x2 at 257.
  - US: 0x10 at play tick 289, 0x1b at 433, and the 0x0 that follows the 0x1b at 503.
- Every other switch lines up: idle, run and walk motions, the fast swings, and returns to idle after them.
- The game rule is plain: a swing starts on the tick its start condition is met, like any other motion switch.
  Play's early start isn't a game rule. It's a play-side difference that the port has to find (see Open questions).

## What the lead is, in B772's terms

Lead = how many frames the game's value is ahead of play's look-ahead-2 value (EU point 3, shift fit of every rally
tick, `scripts/lead_fit.py`):

| Window (rally ticks) | Motion | Lead | Cause |
|---|---|---|---|
| 196–255 | swing 0x12, started one tick early in play | 0 | Rule 2: play +1, so the lead drops from 1 to 0 |
| most idle/run ticks | lined-up motions | 1 | the base lead (see below) |
| 393–423 (396–421 fitted) | speed-4 swing from 390 | 4 | Rule 1: play trails 3, so the lead goes from 1 to 4 |

The US fit (`scripts/us_lead_fit.py` against `fixtures/ai_net_s05.bin`) agrees:

- The early-started 0x10 swing at US play tick 289 shows lead 0 for all of ticks 304–335, the same as the early EU
  swing.
- That is the window B771/M185 calibrated "look ahead 2" on. So the 2 already contains the one-tick early start.
  With both rules in place, a lined-up motion has lead 1 relative to play's current read.

## What the port has to produce

- After Rule 1 and Rule 2, play's motion clock equals the game's on every tick (the anim captures give clip time and
  speed per tick for both builds).
- The racket direction the stand test reads should then be one fixed offset from play's pose for every window. The
  EU fit predicts the lead becomes 1 everywhere. Recalibrate the look-ahead once against both captures. It should be
  the same number in both builds, with no per-swing correction.
- Acceptance: the European bot match gets past point 3's 1585 (game reads +0.2663 there) without the no-contact
  fallback B772 relied on, and the US bot match still plays as recorded at least as far as before.

## Open questions

- Rule 2's cause in play is unknown: why play starts these speed-1 swings one tick before the game does. The
  swing-start condition for frames-to-contact ≥ 8 was not traced. The port session should compare play's swing
  start tick with the anim captures and fix it on the play side. If it turns out to be a game rule that play is
  missing, it comes back through `--spec`.
- The base lead of 1 after both fixes is a prediction from the EU fit. Nobody has run play with both fixes yet.
