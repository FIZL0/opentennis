# M185 spec: what the doubles stand test's scratch direction holds

## Where it matters

In a doubles rally, a computer player's NET/BASE stand routine (the one that picks where the player stands
between shots) runs a chain of spot searches. Each search, when it succeeds, writes the spot it found into a
scratch vector. When every search fails, the routine falls through to the base-line path test, which reads that
scratch vector anyway. Nothing in the stand routine set it on this tick, so it holds whatever the game left in that
memory earlier in the same frame.

The base-line path test uses only one thing from it: the **sign of its z component**. Walking the path entries, an
entry marked as a bounce entry is skipped (go to the next entry) when sign(scratch z) differs from the sign of the
player's side factor. Sign rule: z < 0 counts as −1; everything else (positive, +0, −0, NaN) counts as +1.
x, y and w are never used.

So a port needs, per computer player per tick, just one bit: is the leftover z negative?

## What the game leaves there

The game updates the four players once per frame in court-slot order (slot 0, 1, 2, 3); each player's update runs
its AI (and so the stand routine) near its start. The leftover is written by whatever ran before, at the same
stack depth:

1. **Players updated second, third and fourth (slots 1, 2, 3).** The leftover comes from the *previous player's*
   update that frame. Late in each player's update, the game hands the player's animated body pose to the object
   the player holds (the racket attachment): it copies the 4×4 attachment base matrix of that player's posed model
   and, while doing so, writes **row 0 of that matrix multiplied by the player's mirror factor** (+1 normally, −1
   when the model is drawn mirrored for handedness) into the scratch. Row 0 is a unit direction (w = 0).
   So: scratch z = (previous player's attachment matrix row 0, z) × previous player's mirror factor.
   Checked live on the European bot match: 1785 of 1788 calls.

2. **The first player updated (slot 0).** The previous writer at that depth is the crowd: just before the players,
   the game ticks the gallery's walking spectators, and each walker's tick leaves its own position (x, 0, z, 1).
   The last walker ticked wins. In both recorded matches that is the fourth walker, standing still:
   US bot match (court 10): (8.886850, 0, 12.959484, 1); European bot match: (−7.631587, 0, 19.867296, 1).
   On a few frames (about one in 65, and the first frames after the match loads) the value is (0, 0, 0, 1) instead.
   Every observed case has z ≥ 0, so the first player always reads **+1**.

3. **Exceptions on the tick after the previous player did something with the ball** (rare: one call in several
   hundred):
   - previous player tossed the ball for a serve this tick: the scratch holds the toss's launch velocity
     ((target x − from x)/t, −sqrt(2·g·|Δy|), (target z − from z)/t, 0). Its z sign is the sign of the toss's
     forward drift.
   - previous player's hit started the ball's flight trail this tick: the scratch holds the trail colour as four
     byte values converted to floats (red, green, blue, alpha in 0..255, e.g. (39, 128, 39, 70) or (62, 14, 127, 70)).
     z (blue) is never negative, so it reads **+1**.

   Seen in the European capture: one toss tick, three trail ticks (and one tick with a short non-unit vector of
   positive z, from a shared routine during a hit; see *The unnamed tick*).

## Summary rule for a port

For computer player in slot s at the stand test on a tick:

- s = 0 → +1.
- s ≥ 1 → sign of (z of row 0 of player (s−1)'s held-object attachment matrix this frame × player (s−1)'s mirror
  factor), except: if player s−1 launched its serve toss this tick, use the sign of the toss velocity's z; if player
  s−1's hit started the flight trail this tick, +1.

The pose is the posed model's state *this frame* (after player s−1's animation was advanced in its own update).

## Which matrix: the racket node (answers iteration 2's first open question)

The "held-object attachment matrix" is the world matrix of the character model's node named **`Racket`** (the
first of the named nodes the game looks up on each player model: Racket, Bip01Head, Bip01Neck, …). Row 0 is that
node's local x axis carried into world space.

- World matrix = the node's local matrix, times each ancestor's local matrix up to the model root, times the
  player's placement matrix: the same chain the game uses to hang the serve ball on a bone (play already builds it
  that way for the held ball, with its player matrix whose row 0 already carries the mirror factor). The character's
  size scale, when it isn't 1, scales the placement rows by a positive factor: no effect on the sign.
- The locals are the pose as drawn this frame, taken after all of player s−1's own update has posed it:
  1. the motion step (clip sampling and crossfades, as play's posed nodes already do);
  2. then, while a contact arm solve is running, the arm turn: right hand, right forearm, right upper arm and left
     upper arm each turned by the ramp weight's share of their solved rotations (play's contact arm IK `apply`).
     The hand-off happens after that, so the racket follows the turned arm;
  3. while the **dive** arm solve runs (a computer or human dive that reaches for the ball), the racket node's own
     local also turns about its local y axis by (the dive's racket angle × the same ramp weight), on top of the dive's
     own arm turns. The dive's racket angle is set when the dive's solve starts: the arc cosine of the (clamped)
     dot product of two unit directions in the ground plane, less 180° when it comes out at 90° or more, signed by
     the sign of their cross product's y. Which two directions, and the dive's arm turns, belong to the follow-up's
     own spec. This dive solve is not in play (the N6 dive journal lists its IK step as not ported), so during a
     dive play's racket can differ from the game's. That only matters for the player updated after a diving player,
     on the dive's few frames; a follow-up task covers porting the dive's arm solve.
  4. then the placement matrix is set from the player's current position and facing, and the world matrix built.
- Play's posed-nodes state keeps only a few bones up to date (head, neck, spine, pelvis, toes and their chains).
  The port needs the `Racket` chain kept too (racket, right hand, forearm, upper arm, clavicle and up the spine).
- Order: the game updates player s−1 completely (AI, move, motion, arm turn, racket hand-off) before player s's AI
  runs. So player s's stand test needs player s−1's pose *of this tick*, not of the tick before. Players with a
  higher slot than s are not involved.
- Scratch z = hand(s−1) × (z of row 0 of that world matrix). Only its sign is used, so host arithmetic is fine
  unless the value is within rounding of 0 (never seen: over the 2577 racket-direction calls of the European capture the
  smallest |z| was 0.0079). Bit-exact ps2 arithmetic is still the safer choice where play's chain already uses it.

## Rain courts and courts without walking spectators (second open question)

The leftover in the scratch is plain memory: when nothing writes it in a frame, it keeps the value from the last
frame that did. On a raining court, or with the walkers switched off, the walkers' tick returns before it writes
anything, so slot 0 reads whatever was last written at that depth: the walkers' last position from before (if they
ever walked) or the gallery's other periodic write, (0, 0, 0, 1). Checked live: forcing the European bot match's
court to raining mid-match (600 frames, 217 stand calls per player), slot 0 read (−7.6316, 0, 19.8673, 1) 214 times
and (0, 0, 0, 1) 3 times. No negative z anywhere. So slot 0 reads **+1** on every court; nothing to model.

## The skip flag (fourth open question)

The pose hand-off is skipped while the game's scene-start flag is set. The main loop sets it when a new scene is
requested and the match's own setup clears it before any player is updated in play; it is never set while a point
is being played. The stand test therefore never sees the skip: nothing to model.

## The unnamed tick (third open question)

European frame 38329, slot 2: (0.0221, 0.1487, 0.0035, 0.1453). That frame player 1's swing was passing through
contact (its racket row 0 went from (−0.056, −0.299, 0.953) the frame before to (0.828, −0.082, 0.555) the frame
after), and a shared library routine called during the hit left this short non-unit value instead. Its z is
positive, and the racket's z on both neighbouring frames is positive, so reading the racket gives the same sign
(+1). Nothing extra to model; if a port ever disagrees on a contact tick, this writer is the first suspect.

## Open questions

none
