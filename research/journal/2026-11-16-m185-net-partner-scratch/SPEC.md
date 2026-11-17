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
   positive z, writer not named; see open questions).

## Summary rule for a port

For computer player in slot s at the stand test on a tick:

- s = 0 → +1.
- s ≥ 1 → sign of (z of row 0 of player (s−1)'s held-object attachment matrix this frame × player (s−1)'s mirror
  factor), except: if player s−1 launched its serve toss this tick, use the sign of the toss velocity's z; if player
  s−1's hit started the flight trail this tick, +1.

The pose is the posed model's state *this frame* (after player s−1's animation was advanced in its own update).

## Open questions

- Play doesn't model the posed animated skeleton. The port needs the z of the held-object attachment matrix's first
  row for the previous player. Unknown how directly play's existing animation state gives it; if it doesn't, a
  recording of that value per player per tick (from the bot matches) is the reference to match.
- Rain courts and courts with the walking gallery switched off: the walkers write nothing there; what the first
  player then reads was not observed (no capture). Only slot 0 is affected.
- One European tick (slot 2) held (0.0221, 0.1487, 0.0035, 0.1453): positive z, writer not named.
- The player update skips the pose hand-off when a global "presentation" flag is set; what is left then was not
  observed (it was never set during the matches).
