# B675 spec: players' heads track the ball

This describes what the original does. It is written from the decompile, and the timing and angles were checked
against a live recording of the bot-only doubles game (slot 5, 3000 frames, three points). The Port phase works
from this file.

## What it is

Each player turns one bone, the **head** (the skeleton's Head joint, the child of Neck), toward the ball. The neck,
the eyes and the face tracks are not touched. The whole effect is a rotation written into the head's local matrix
after the motion clip has posed it, followed by a recompute of the world matrices of the head and everything under it.

**It feeds gameplay.** Body-hit detection (a ball hitting a player) uses the head's world x-row to place the head
sphere (0.15 m back along that row, radius 0.25). The look rotation changes that row, so the port must be bit-exact
(`hst_sim::ps2` arithmetic, the same rotation builders as the world/prop matrices). The head's world *position* is
not changed, so cameras that read only the head position are not affected.

Body-hit runs during the player's state update, and the look runs at the very end of the player's tick. So body-hit
always sees **the previous tick's turned head**. Our current body-hit uses the clip pose with no look, so it is not
exact once the look is active.

## Where it runs in the tick

Each player is ticked in this order: state update (rally/serve/post-point logic, which includes body-hit), motion
advance, controllers, arm IK, and placing the model at the player matrix. Then comes **pose finalize**, which runs
the head look last. Pose finalize is skipped for every player on a tick when the game is switching scenes.

Slow motion (the exhibition's "slow motion" special rule, the only thing that turns it on in a match) adds
in-between frames. On such a frame each player re-poses at the in-between fraction and then runs pose finalize as
well, without the scene-switch check, so the look runs on in-between frames too (same gating, same routine, with
the slow-motion factor below). Outside that rule there are no in-between frames.

## Constants

| meaning | value |
|---|---|
| tracking enabled (global switch) | on |
| eye height above the head bone origin, along world up (game space is y-down, so the eye is at −0.15 in y) | 0.15 m |
| nod window, relative to the rest pose | 0° … +30° |
| turn window, relative to the rest pose | −50° … +50° |
| re-seed threshold (how far the rest forward may move between motions) | 15° |
| maximum change per tick, per angle | 7° (0.122173 rad) |
| release target (used as is: not transformed, not remapped) | (0, 0, 1) |
| weight (players) | 1.0 |
| slow-motion factor | 1 normally; 1/(n+1) in f32 (1.0 divided by the integer n+1 converted to float) while slow motion is on, n being the slow-motion clock's current divisor (the same one that paces the ball's in-between frames) |
| shared contact hold | 15 ticks |
| "just hit" window | 6 ticks after contact |

The degree constants are stored as degrees and turned into radians where they are used: degrees × 0.017453292 as
one f32 multiply (chopped). The −50° bound is the negation of 50 × 0.017453292, the +50° bound the negation of that.
The tick step is (7 × 0.017453292) × slow-motion factor, in that order.

## Angle helpers

- **The remap.** Only the ball target is remapped: the head-local target point l becomes v = (l.y, l.x, −l.z).
  The forwards F and R and the final forward are world-space rows and are used as they are, with no remap. The
  release vector is used as it is too.
- **nod(v)** = atan2(−v.y, sqrt(v.x·v.x + v.z·v.z)). **turn(v)** = atan2(v.x, v.z). The sum of squares is two FPU
  multiplies and one FPU add (chopped, x term first), then the FPU square root; atan2 is the game's
  (`hst_sim::libm`).
- **wrapped difference(a, b)**: with π = 3.1415927 and 2π = 6.2831855 as f32 constants, d = (a + π) − (b + π),
  three FPU ops in that order (both sums first, then the subtraction). Let p = d + 2π and m = d − 2π. If |p| ≤ |d| or
  |m| ≤ |d|, the result is m when |m| ≤ |p|, else p; otherwise it is d. (So on an exact tie m wins over p, and either
  wins over d.) "The difference of two vectors' angles" means this, applied to the first vector's angle and the
  second's, each angle computed first.

## Per-player state

- **initialised** flag.
- **re-seed pending** flag.
- **snap** flag (skip the rate limit once).
- **previous applied nod / turn**: the angles applied last tick, after the weight.
- **recorded rest nod / turn**: the rest forward's angles, taken on the ticks when the clip-end counter is 0.
- **last final nod / turn**: the forward angles of the head as finally written.
- **clip-end counter.**
- On the player side: a **latch** ("tracking is on for this motion").

**Full reset**: initialised off, last-final angles 0, counter 0, re-seed pending on, and snap = whether the game is
switching scenes right now.

## The turn routine (one call, with a target: the ball, or "release")

The **parent′** frame is the head's parent world matrix. When the model is mirrored, the first element of each of
its three rotation rows (rows 0–2; not the translation row) is multiplied by −1 first. A model is mirrored when its
character is left-handed (the character's handedness entry says left; Carol and Will on the disc), so this is the
lefty case. Steps 1, 10 and 11 use parent′; step 4 does not (see there).

All matrix products are VU0 multiply-accumulate chains, row vectors, as `hst_sim::world::mat_mul` (A × B = every row
of A through B). "Inverse" is always the rigid inverse of `hst_sim::world::inverse` (transposed 3×3, translation row
−(t · Rᵀ), the translation row keeping t's w).

1. **Animated forward** F = row 2 (z row) of (the head's local matrix from the clip this tick × parent′).
   **Rest forward** R = row 2 of (the model's rest local for the head × parent′).
2. **Clip-end counter.** If the clip's length ≤ its time (a motion with no clip counts as length 0), the counter
   goes up by one; when it was already above 0 before this tick, the previous applied nod/turn are first set to 0.
   Otherwise the counter is set to 0. So once a clip sits on or past its end for a second tick, the rate limit is
   measured from the clip pose every tick: the head can lean at most 7° off the clip pose and swings back to within
   7° of it at once. While the counter is above 0 the rest angles are not re-recorded (step 11).
3. If the global switch is off, or *initialised* is off: set *initialised* and skip to step 11 (bookkeeping). The
   head keeps its clip pose this tick.
4. **Target vector**, in head-local terms:
   - Tracking: let H = clip local × **parent** (the true parent, not parent′; for a right-hander the two are the
     same, for a lefty this is the one place the unmirrored parent is used), and Q = inverse(H). Shift Q's
     translation row by the eye: each of its four lanes becomes row3 + ((−row1) × (−0.15)) (an FPU multiply, then an
     FPU add). Transform the ball's position (x, y, z, w = 1, as the ball stores it) through Q's rows 0–2 and the
     shifted row 3 (one VU0 chain). This is the ball relative to an eye 0.15 m above the head origin (world up = −y),
     in head-local terms. Remap the result.
   - Release: the fixed vector (0, 0, 1), as it is: no transform, no remap. So the wanted nod is atan2(−0, 1) = −0
     and the wanted turn is atan2(0, 1) = 0, i.e. the clip pose.
5. **Re-seed**, only when *re-seed pending* is on (the first call after a motion change). Clear it, then:
   - previous applied nod = −wrapped difference(last final nod, nod(F))
   - previous applied turn = wrapped difference(last final turn, turn(F))
   - If nod(R) or turn(R) has moved more than 15° (wrapped) from the recorded rest angles, both previous applied
     angles are set to 0.

   This makes the head carry on from where it was last tick, now measured against the new clip's pose, unless the
   new clip's rest frame moved a lot.
6. **Windows.** These are measured from rest and shifted so they always contain 0 (the clip's own pose):
   - Nod: e = nod(R) − nod(F) (wrapped difference). excess = e − 30° if e > 30°, e − 0° if e < 0°, else 0.
     s = (−e) + excess. The window is [0° + s, 30° + s].
   - Turn: d = turn(R) − turn(F) (wrapped difference), e = −d. excess = e + 50° if e < −50°, e − 50° if e > 50°,
     else 0. The window is [(−50° + d) + excess, (50° + d) + excess].

   (Each bracket is the FPU add order: the nod adds the limit to the sum s, the turn adds d to the limit first.)

   Quirk: when the clip already turns the head past a limit, the window is pinned with one end at 0. That lets the
   look go up to twice the limit the other way. The recording shows turn reaching −100° (−1.745 rad).
7. The wanted nod = nod(target), and the wanted turn = turn(target). Clamp each into its window: below the low end
   → the low end; else above the high end → the high end; else as is.
8. **Rate limit.** If *snap* is on, clear it and skip this step. Otherwise, with step = (7° in radians) × factor,
   each angle is clamped the same way into [previous − step, previous + step] (both bounds one FPU op).
9. Multiply each angle by the weight (1.0) and store it as the new previous applied angle.
10. **Apply.** W = clip local × parent′ (the matrix F came from). World′ = rotY(nod) × (rotX(turn) × W), evaluated
    in that order: first rotX(turn) × W, then rotY(nod) × that. The rotations are the builders of
    `hst_sim::world::rot_x` / `rot_y` (the game builds each one by passing the identity through it, which only
    normalises the signs of zeros). The new head local = world′ × inverse(parent′). Store it as the head's local,
    then recompute the world matrices of the head and of every node under it.
11. **Bookkeeping.** If the counter is 0, record nod(R) and turn(R) as the rest angles. Always record nod and turn of
    row 2 of the head's world-space matrix as it stands now as the last final angles: row 2 of world′ when step 10
    ran, row 2 of W (that is, F) when step 3 skipped. Not remapped, and not re-derived from the stored local.

Note the axes: in head-local terms, "turn" is a rotation about the head's x axis and "nod" one about its y axis.
That follows from the skeleton's bone axes. The remap in the helpers is what lines them up.

## When a player calls it (gating, each tick)

Definitions on the player's current motion:
- **stroke**: the motion is a groundstroke, volley, soft shot, receive or smash (motion ids 0x10–0x1f), or a serve
  swing or whiff (0x25–0x2a).
- **dive**: a stroke on the dive branch. **whiff**: 0x27–0x2a. **smash**: 0x1f.
- **since**: the frames since this player's last ball contact.
- **server before the first shot**: this player is the server and the rally has had no shot yet.

First, the **shared contact hold**. This is a single counter for the whole match, used only in doubles. On the tick
before any player's contact (that player's swing is in its pre-contact step with the stroke countdown at 1, either
in a rally swing or in a serve), the hold restarts at 15. Each tick of its owner it first counts down by one (if
above 0), then, in doubles, restarts at 15 if any player is in that pre-contact step. It is set to 0 when the match
starts, on each change-of-ends set-up and on each serve set-up. Its enable flag is on for the whole match: the only
time it is off is inside the owner's own handling of a hit, where the owner re-runs its tick once with the flag off,
so that extra run neither counts the hold down nor restarts it. A port can drop the flag and simply not touch the
hold in that re-run (or not have the re-run at all, if nothing else in it is ported). **While the hold is above 0
when a player's pose finalize runs, that player's look does not run at all**: no call, the head shows the clip,
and the state is untouched.

Then, per player:

- **Latch off:**
  - not a stroke → no call this tick.
  - a stroke that is not a dive or whiff, with since ≥ 6 → **latch on**, then track (below), this same tick.
  - otherwise (since < 6, or a dive, or a whiff), checked in this order:
    - server before the first shot and since < 6 → no call.
    - dive or whiff (any since) → no call.
    - smash (since < 6 here) → no call.
    - else → call with **release** (ease toward the clip pose at up to 7°/tick).
- **Latch on:** call with **track the ball**, except in doubles while the point is still live (match phase is not
  "point over"). There, if this player is nearer the net than their partner (|own z| < |partner z|, strictly; the
  partner is the player in seat (seat + 2) mod 4), the ball is on this player's side of the net (the signs of the
  ball's z and the player's z agree, z = 0 counting as positive) and the ball is deeper than this player
  (|own z| < |ball z|, strictly), it calls with **release** instead. With fewer than three players this check is
  skipped.

**On every motion change:** the latch is cleared and *re-seed pending* is set. In addition:
- If the match phase is at least "rally/serve under way" (phase ≥ 2), and (the player is not the server before the
  first shot, or this is singles), and the new motion is not a dive and not a smash, then:
  - if the change is a blend (a crossfade from the old motion), and the new motion is either a non-stroke or a
    non-whiff stroke → **latch on**. Otherwise the latch stays off.
  - no full reset.
- Otherwise → **full reset**.

This is how the other three players pick up tracking while someone else swings: their blended run or stand changes
during the rally switch the latch on.

## What the recording shows (slot 5, bot-only doubles)

- During the serve wait (before the serve) there are no calls. The heads play their clip.
- After a contact, the hitter's latch comes on at contact + 15 ticks (the hold) and stays on while running and
  standing. It goes off on the player's next stroke start and comes back 15 ticks after that contact.
- The receivers stay unlatched until the first shot. Then their blended motion changes latch them.
- The post-point motions (won/lost, team reactions) are not strokes and are not blended into a tracking state, so
  the latch stays off after the point. **Between points the heads do not look at anything.** They play the clip,
  and the next point starts from a full reset.
- The 7°/tick steps are visible in the applied angles. The clip-end counter stayed 0 throughout.

## Port notes

- The state lives with each player in the sim (`hst-sim`). The look is gameplay, so it must go through the replay
  harness. Mode-off replay tests stay exact only if the port is exact, and body-hit should then use the turned head
  from the previous tick.
- The ball target is the ball position as the player tick sees it (the ball's current position at the end of the
  player's tick).
- The renderer just draws the head local the sim wrote. No separate smoothing is needed in Bevy.
- A lefty (mirrored) player: parent′ (rotation rows' first elements negated) for F, R, the apply and the
  bookkeeping; the true parent for the ball target. The angles and windows are not mirrored.
- Slow motion only exists in the remaster on the Offbeat Rules concept branch, which paces the clock instead of
  adding sim in-between frames. On main the factor is always 1; if that rule ever runs real in-between frames, the
  look runs on them too (see *Where it runs in the tick*).
- The look runs only from the player's tick and the slow-motion in-between frame. Our instant replays and other
  presentation re-plays must draw the head locals the sim stored, never run the look again (it would advance the
  look state).

## Answers to the first round's questions

- **Lefty.** Read from the decompile: see parent′ and step 4 above (the target goes through the true parent, the
  rest through parent′). Not recorded live: the bot-only save state has no left-hander. The port phase proves it
  without a recording by running the game's own routine on the function oracle with the model's mirror flag set
  (state from a RAM dump, the ball position and matrices as arguments), ≥10k inputs; if the oracle lacks a VU0 macro
  op the routine needs, add it in ps2-oracle (AGENT.md).
- **The hold's enable flag.** On for the whole match; off only inside the owner's re-run on a hit, where the hold
  is not touched. Also new: the hold is cleared at match start, change-of-ends set-up and serve set-up.
- **Slow-motion factor and replays.** Slow motion is the exhibition's slow-motion rule; it divides the step by n+1
  and its in-between frames run the look too. Replays: see port notes.
- **Clip-end counter.** Described in step 2: after a clip has sat at its end for a second tick the head is held
  within 7° of the clip pose. The port can force it in a unit test (a clip whose time is at its length).
- **Wrapped-difference rounding.** The op order is in *Angle helpers* (both π sums first, then the subtraction,
  π and 2π as f32 constants; the tie rule there replaces the first round's, which had it backwards). The port's
  test against `context/b675/run1.jsonl` (copied into `fixtures/b675/` with the test) checks it bit for bit.

Also corrected from the first round: F, R and the final forward are not remapped (only the ball target is), the
release vector is used raw (not remapped, which would have meant a 180° turn), the last final angles come from
world′ rather than the stored local, the doubles release check's ties, and the latch-off gating (a dive or a whiff
never calls, whatever the frames since contact).

## Open questions

none
