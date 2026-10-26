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

## Constants

| meaning | value |
|---|---|
| tracking enabled (global switch) | on |
| eye height above the head bone origin, along world up (game space is y-down, so the eye is at −0.15 in y) | 0.15 m |
| nod window, relative to the rest pose | 0° … +30° |
| turn window, relative to the rest pose | −50° … +50° |
| re-seed threshold (how far the rest forward may move between motions) | 15° |
| maximum change per tick, per angle | 7° (0.122173 rad) |
| release target, in head-local terms (straight ahead) | (0, 0, 1) |
| weight (players) | 1.0 |
| slow-motion factor | 1 normally; 1/(n+1) when the game's slow-motion timer is on with divisor n (it was off in every match tick recorded) |
| shared contact hold | 15 ticks |
| "just hit" window | 6 ticks after contact |

All degree constants are stored as radians (degrees × π/180 in f32).

## Angle helpers

- **The remap.** A head-local vector l is remapped to v = (l.y, l.x, −l.z) before any angles are taken. Every
  angle below is taken from such a remapped vector.
- **nod(v)** = atan2(−v.y, hypot(v.x, v.z)). **turn(v)** = atan2(v.x, v.z). Both use the game's atan2 (`hst_sim::libm`)
  and its sqrt.
- **wrapped difference(a, b)**: d = (a + π) − (b + π) in f32. Of d, d + 2π and d − 2π, take the one with the smallest
  magnitude. On a tie, prefer d, then d + 2π, then d − 2π. "The difference of two vectors' angles" means this,
  applied to the first vector's angle minus the second's.

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

The **parent′** frame is the head's parent world matrix. When the model is mirrored (a left-handed character drawn
mirrored), its x column is negated first. Every step below uses parent′.

1. **Animated forward** F = row 2 (z row) of (the head's local matrix from the clip this tick × parent′).
   **Rest forward** R = row 2 of (the model's rest local for the head × parent′).
2. **Clip-end counter.** If the clip's time ≥ its length, the counter goes up by one. On every such tick except the
   first one, the previous applied nod/turn are set to 0. Otherwise the counter is 0.
3. If the global switch is off, or *initialised* is off: set *initialised* and skip to step 11 (bookkeeping). The
   head keeps its clip pose this tick.
4. **Target vector**, in head-local terms:
   - Tracking: take the ball's world position minus the eye point (the head's world origin with 0.15 m added in
     world up). Bring it into the frame of (the head's clip local × parent′) by the inverse of that matrix, then remap it.
   - Release: the fixed (0, 0, 1), remapped the same way.
5. **Re-seed**, only when *re-seed pending* is on (the first call after a motion change). Clear it, then:
   - previous applied nod = −wrapped difference(last final nod, nod(F))
   - previous applied turn = wrapped difference(last final turn, turn(F))
   - If nod(R) or turn(R) has moved more than 15° (wrapped) from the recorded rest angles, both previous applied
     angles are set to 0.

   This makes the head carry on from where it was last tick, now measured against the new clip's pose, unless the
   new clip's rest frame moved a lot.
6. **Windows.** These are measured from rest and shifted so they always contain 0 (the clip's own pose):
   - Nod: e = nod(R) − nod(F) (wrapped difference). excess = e − 30° if e > 30°, e − 0° if e < 0°, else 0.
     The window is [0° − e + excess, 30° − e + excess].
   - Turn: d = turn(R) − turn(F) (wrapped difference), e = −d. excess = e + 50° if e < −50°, e − 50° if e > 50°,
     else 0. The window is [−50° + d + excess, 50° + d + excess].

   Quirk: when the clip already turns the head past a limit, the window is pinned with one end at 0. That lets the
   look go up to twice the limit the other way. The recording shows turn reaching −100° (−1.745 rad).
7. The wanted nod = nod(target), and the wanted turn = turn(target). Clamp each into its window.
8. **Rate limit.** If *snap* is on, clear it and skip this step. Otherwise each angle is clamped to within
   previous ± 7° × slow-motion factor.
9. Multiply each angle by the weight (1.0) and store it as the new previous applied angle.
10. **Apply.** World′ = rotY(nod) · rotX(turn) · (clip local × parent′), using row vectors: the turn about x is
    applied first, then the nod about y. These are the same rotation builders as `hst_sim::world::rot_x` / `rot_y`.
    The new head local = world′ × inverse(parent′). Store it as the head's local, then recompute the world matrices
    of the head and of every node under it.
11. **Bookkeeping.** If the counter is 0, record nod(R) and turn(R) as the rest angles. Always record the nod and
    turn of the final head forward (row 2 of the local written this tick, × parent′, remapped) as the last final angles.

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
in a rally swing or in a serve), the hold restarts at 15. It counts down by one each tick while its enable flag is
on (the flag was on throughout the recording). **While the hold is above 0, no player's look runs at all**: no
call, the heads show the clip, and the state is untouched.

Then, per player:

- **Latch off:**
  - not a stroke → no call this tick.
  - server before the first shot and since < 6 → no call.
  - dive, whiff or smash with since < 6 → no call.
  - any other stroke with since < 6 → call with **release** (ease toward the clip pose at up to 7°/tick).
  - a stroke that is not a dive or whiff, with since ≥ 6 → **latch on**, then track (below).
- **Latch on:** call with **track the ball**, except in doubles while the point is still live (match phase is not
  "point over"). There, if this player is nearer the net than their partner (smaller |z|), the ball is on this
  player's side of the net (same z sign) and the ball is deeper than this player (larger |z|), it calls with
  **release** instead.

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
- A lefty (mirrored) player uses parent′ with the negated x column. That is the only handedness change. The angles
  and windows are not mirrored.

## Open questions

- Left-handed (mirrored) players were not in the recorded game. Parent′ with the negated x column is from the
  decompile only. Verify with a lefty (Carol or Will) recording.
- The enable flag of the shared hold was always on. When (if ever) it is off is not known. Treat it as on.
- The slow-motion factor was never active in a match tick. Replays and slow-motion probably re-pose separately; check
  that replays don't re-run the look.
- The clip-end counter branch (zeroing the previous angles after a clip ends without a motion change) was never
  seen in the recording.
- The exact f32 rounding of the wrapped difference ((a+π)−(b+π)) needs confirming in the port, against a recording of
  the applied angles, ≥ 1 point (`research/b675_look_rec.py` logs them).
