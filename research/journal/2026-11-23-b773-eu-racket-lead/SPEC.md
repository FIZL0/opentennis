# B773 spec: why the European racket lead changes per swing

What the doubles stand test reads as player 1's leftover racket direction, when in the tick that pose is taken, and
why play's look-ahead-2 model is off by a different amount per swing. Both builds behave the same way. The European
rally just happens to contain the swing that shows it most.

## What the game reads

- The leftover z the stand test reads on tick T is the racket direction (racket node's world x axis, z component,
  times the hand sign) of the player's pose at the game's own motion clock on tick T: the clip time the motion
  player has sampled that tick. EU point 3: on rally ticks 405–411 the game's value equals play's pose at the same
  clip time (tick 409: clip time 25 in both).
- So the AI reads this tick's pose. The game has no extra pose pass and no look-ahead. The difference from play
  comes from play's motion clock running out of step with the game's, and from play's fixed look-ahead (*Play's
  stored pose is one step behind, not two*). Both builds step motions once per game tick.
  The 50 Hz build doesn't step motions at a different rate here.


## When in the tick the pose is taken

Each player's update runs in court-slot order and does these in order:

1. the computer player's AI (a human's pad instead), then its mode's movement and motion commands. A newly decided
   swing, stance or run starts here;
2. one motion step: the current motion is sampled at its clock, then the clock moves on by the speed, and the
   crossfade counts down;
3. the pose's extra layers (the contact arm turn among them) and the crossfade's mix of the outgoing motion;
4. the player's placement for this tick: position and facing after this tick's move;
5. the racket hand-off. The step and the placement mark the skeleton as changed, so it is evaluated afresh here from
   the pose of steps 2–3 under the placement of step 4. The racket node's matrix, with its x row times the hand sign,
   goes to the racket model through the scratch slot. That copy is what the next player's stand test reads;
6. later in the same update: the toss (on a toss tick its launch velocity overwrites the slot) and the per-frame swing
   update (*The per-frame swing update runs after the motion step*).

So player 2's AI on tick T reads player 1's pose after player 1's own motion step on tick T, under player 1's facing for
tick T. That includes a motion player 1's own update started on tick T: its first sample (clip time 0) with that
tick's crossfade mix, or a cut. Nothing comes from an earlier tick and nothing is looked ahead.

Measured (game's read against the game's own sampled clip time in the same frame; the anim capture records a frame's
state at the start of the next frame):

- EU rally 409: clip time 25, 411: 27, 396: 12 (the speed-4 swing), 289–290: 3, 4 (a lined-up run), 200: 20 (the
  queued 0x12 swing).
- US play ticks 26 (idle, clip time 28) and 310 (the 0x10 swing, clip time 20).
- EU rally 390: player 1 cuts from 0x1 to the speed-4 swing 0x10 that tick. The game's read on 390 (−0.7529) is
  already the swing at clip time 0. Play's value, carried on from the old motion, is −0.8270.
- US play tick 888: player 1 switches from 0x1f to 0x5 on the same tick in the game and in play. The game's read on 888
  (0.4028) is already 0x5 at clip time 0 with its crossfade from 0x1f; play's look-ahead value (0.4855) is the old
  motion run on. On 890 the game reads 0x5 at clip time 2 (−0.0784); play's look-ahead 2 gives clip time 3 (−0.0913),
  and look-ahead 3 reaches further still. The port attempt's parting at 890 comes from this.
- When the game's skip-drawing flag is set, the hand-off doesn't run (and the motion step doesn't sample), so the slot
  keeps whatever its last writer left (the M185 spec's writers).

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
  - EU: the swing to motion 0x12 at rally tick 180. The switch to 0x2 at 257 is not an early start of its own:
    it is the stance that follows the swing when it ends (see *After a swing ends*).
  - US: 0x10 at play tick 289, 0x1b at 433, and the idle 0x0 that follows the 0x1b swing at 503. That return comes
    early because the early swing ends early.
- Every other switch lines up: idle, run and walk, the fast swings, and the returns after lined-up swings.
- EU: the game's swing to 0x10 at rally tick 539 is also a queued speed-1 swing. B772's lead-0 window 555–589 lies
  inside it. Play had already left the recording by then (it diverges after 1585), so that start couldn't be
  compared directly. Lead 0 is what Rule 2 predicts there.

## After a swing ends

- In stand mode, once a swing clip has run out, the player goes back to a ready stance: the backhand turn (0x2) or
  the forehand turn (0x1), chosen by which side of the ball's line the player stands on. The square stand is 0x0.
- EU: the 0x12 swing starts at rally tick 180 (clip time 0) and runs 77 ticks to tick 256 (last sampled clip time 76).
  0x2 starts at 257 at clip time 0, with a 7-tick crossfade from the swing held at clip time 77.
- Play: 0x12 starts at 179 (play tick 1353), runs the same 77 ticks to 255 (play tick 1429), and 0x2 starts at 256
  (play tick 1430).
- The swing has the same length in both. Play's 0x2 is early only because its 0x12 started a tick early (Rule 2).
  It is the same knock-on as the US idle 0x0 after the early 0x1b swing at 503.
- Moving the per-frame swing update after the motion step makes the swing start at 180 and end at 256, so the stance
  follows at 257 with no extra change.
- Port check: against the EU anim capture, the stance read one tick later must still pick 0x2 at 257, as the game
  does, and the crossfade must match.


## Play's stored pose is one step behind, not two

- Play poses after its motion step, at that step's sampled clip time, and the after-pose store keeps that pose. The AI
  reads it on the next tick. So the stored pose is exactly one motion step behind the tick the AI is in, and with
  play's clock equal to the game's, one step on from it is the game's read when player 1's motion didn't change
  that tick.
- The shift fits (`research/b773_lead_fit.py`, `research/b773_us_lead_fit.py`) give s, the play tick (relative to
  the same tick) whose look-ahead-2 value equals the game's read:

| Window | Play's clock against the game's | s | Look-ahead that fits |
|---|---|---|---|
| lined-up motions (EU 289–290, 302–306 and the other idle/run windows) | equal | −1 | 1 |
| queued speed-1 swings play starts a tick early (EU 194–255, US 304–335) | 1 frame ahead | −2 | 0 |
| speed-4 swing from EU 390 (396–421) | 3 frames behind | +2 | 4 |
| speed-8/3 swings | 1.667 frames behind | none (no whole-tick shift fits) | none; lines up once Rule 1 is ported |
| US probe's opening idle (play ticks 26–200) | 3 frames behind | +2 | 4 |

- s = −1 means the game's read equals the value play's AI read one tick earlier: look-ahead 2 reaches one frame too
  far. The earlier reading of the same numbers ("base lead 1, so look-ahead 3") had the direction backwards.
- M185's look-ahead 2 was set in windows where play's clock was off (the rows above), and only the sign of z is read,
  so a wrong look-ahead passes for long stretches. The held-ball reason given for "two ticks" doesn't apply: play's
  held ball has nothing to do with when the stand test's pose is taken.
- The US opening idle row comes from the probe's start: play's idle clock starts at 0 there while the game's idle was
  already 3 frames in. That is a start-state difference in the test harness, not a timing rule (follow-up filed).

## What the port has to produce

- Run play's per-frame swing update (countdown, queued start, fast-speed reset) after the motion step, so its
  changes show from the next step (Rules 1 and 2). The decision-time fast start stays where it is. Play's motion
  clock then equals the game's on every tick of both anim captures, apart from the US probe's opening idle offset.
- Drop the fixed look-ahead. The steps below are how play rebuilds the game's same-tick pose from what it keeps, not
  something the game does. The leftover computer player i reads on tick T is player i−1's racket z from player
  i−1's pose of tick T:
  - start from player i−1's stored pose of tick T−1;
  - apply player i−1's own motion command for tick T, the way play's motion setter does in that tick: a new motion
    starts at clip time 0 with its crossfade or cut, a restart or speed change takes effect;
  - run exactly one motion step (sample, then advance; crossfade count and mix);
  - use player i−1's facing for tick T and its arm turn (and a dive's racket turn) for tick T.
- Players run in court-slot order, so all of player i−1's update for tick T comes before player i's AI. In play the
  motion commands become motions only after every AI has run, so the port has to work out player i−1's tick-T motion
  at player i's AI time. It can't wait for the motion step: player i's decision depends on the read.
- With no motion change on tick T this is the stored pose plus one step (look-ahead 1). On a switch it is the new
  motion's first sample.
- The toss-tick, hit-tick and first-player overrides stay as they are.
- Acceptance:
  - EU bot match: past point 3's 1585 (the game reads +0.2663 there, clip time 27) without the no-contact fallback.
  - US bot match: at least as far as before the port attempt (2354).
  - Shift fit: the value play's AI reads on tick T equals the game's read on T on
    every tick where slot 2 stored a racket direction, including the switch ticks (EU 390). The exception is the US
    probe's opening idle, until its start offset is fixed.

## Open questions

none
