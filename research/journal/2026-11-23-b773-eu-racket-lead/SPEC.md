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

## The per-frame swing update runs after the motion step

- Each player has a contact countdown: the frames left to contact. A per-frame swing update decrements it, starts
  a queued swing, and turns a fast swing's speed back to 1. In the game this update runs after that tick's motion
  step. Anything it changes (a new motion, a new speed) first takes effect on the next tick's step. Play does the
  same work before the step, so its changes land one step early. That single ordering difference produces both
  effects below.

## Rule 1: a fast swing keeps its speed until frame 8

- When the stroke is decided with few frames left, the swing motion starts immediately at
  speed = 8 / frames-to-contact (speed 1 when frames-to-contact is below 1). That start happens in the decision,
  before the step, and lines up in play and the game.
- The game keeps that speed for exactly frames-to-contact motion steps, counting the start tick's step. The clip
  clock reaches exactly 8 on the contact tick, and the next step is speed 1.
  - Speed 4 (2 frames): steps 4, 4, 1, so the clock goes 0 → 4 → 8 → 9.
  - Speed 8/3 (3 frames): steps 2.667, 2.667, 2.667, 1, so the clock goes 0 → 2.667 → 5.333 → 8 → 9.
  - Speed 2 (4 frames): four steps of 2, then 1.
  - Slower starts keep their speed for longer.
- Play resets the speed one step early: 0 → 4 → 5 at speed 4, 0 → 2.667 → 5.333 → 6.333 at speed 8/3. For the rest
  of the swing it trails the game by 3 frames (speed 4) or 1.667 frames (speed 8/3).
- Seen on every fast start in both anim captures, in the first four steps (`research/b773_fast_starts.py`).
- Examples:
  - EU rally tick 390, speed-4 swing: game clock 0, 4, 8, 9; play clock 0, 4, 5, 6.
  - US phase 3 tick 429: game clock 0, 2.667, 5.333, 8, 9; play clock 0, 2.667, 5.333, 6.333.

## Rule 2: a queued swing starts when the countdown reaches 8

- When a forehand or backhand is decided with more than 8 frames left, the game does two things:
  - it plays a ready motion right away at speed 1, one of two depending on which swing was chosen;
  - it queues the swing.
- The per-frame update starts the queued swing at speed 1 on the frame where the decremented countdown equals 8.
  It runs after the step, so the swing shows from the next tick's step.
- Play starts these one tick early.
  - EU: the swing to motion 0x12 at rally tick 180 and the switch to 0x2 at 257.
  - US: 0x10 at play tick 289, 0x1b at 433, and the idle 0x0 that follows the 0x1b swing at 503. That return comes
    early because the early swing ends early.
- Every other switch lines up: idle, run and walk, the fast swings, and the returns after lined-up swings.
- EU: the game's swing to 0x10 at rally tick 539 is also a queued speed-1 swing. B772's lead-0 window 555–589 lies
  inside it. Play had already left the recording by then (it diverges after 1585), so that start couldn't be
  compared directly. Lead 0 is what Rule 2 predicts there.

## What the lead is, in B772's terms

Lead = how many frames the game's value is ahead of play's look-ahead-2 value (EU point 3, shift fit of every rally
tick, `research/b773_lead_fit.py`):

| Window (rally ticks) | Motion | Lead | Cause |
|---|---|---|---|
| 196–255 (and 555–589) | queued speed-1 swing, started one tick early in play | 0 | Rule 2: play one frame ahead, so the lead drops from 1 to 0 |
| most idle/run ticks | lined-up motions | 1 | base lead: play's look-ahead 2 reads one frame short |
| 393–423 (396–421 fitted) | speed-4 swing from 390 | 4 | Rule 1: play trails 3, so the lead goes from 1 to 4 |
| speed-8/3 swings | | about 2.667 | Rule 1: play trails 1.667. No whole-tick shift fits, so those ticks show no match |

The US fit (`research/b773_us_lead_fit.py` against `fixtures/ai_net_s05.bin`) agrees:

- The early-started 0x10 swing at US play tick 289 shows lead 0 for all of ticks 304–335, the same as the early EU
  swing.
- That is the window B771/M185 calibrated "look ahead 2" on. So the 2 already contains the one-tick early start.
  With both rules in place, a lined-up motion has lead 1 relative to play's current read.

## What the port has to produce

- Run play's per-frame swing update (countdown, queued start, fast-speed reset) after the motion step, so its
  changes show from the next step. The decision-time fast start stays where it is. Play's motion clock should then
  equal the game's on every tick of both anim captures (clip time and speed per tick).
- With the clocks equal, every window has the same lead of 1. Play's look-ahead goes from 2 to 3, once, in both
  builds. US: M185 calibrated 2 on the early-started 0x10 swing (lead 0), so that window moves to lead 1 too. No
  per-swing correction.
- Acceptance: the European bot match gets past point 3's 1585 (game reads +0.2663 there) without the no-contact
  fallback B772 relied on, and the US bot match still plays as recorded at least as far as before.

## Open questions

- EU's early 0x2 at rally tick 257 is a ready motion, which play starts at the stroke decision rather than in the
  per-frame update. Check in the port, against the EU anim capture, whether moving the per-frame update fixes it
  too: the decision for that stroke may itself run in the after-step update. If it doesn't, bring the case back
  through `--spec`.
