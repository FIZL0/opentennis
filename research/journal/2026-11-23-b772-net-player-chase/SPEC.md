# B772 spec: why the EU net player chases at point 3's tick 1585

The European bot match (M183a1), point 3's rally, recorded tick 1585: the near-side computer net player (player index 2)
runs from a standstill toward the after-bounce stand spot of predicted-path entry 39, and the game draws 4 AI values.

## What the game does (no new rule)

The game uses the rally routine's existing fallback, the one that runs when no tier finds a reachable contact. The port
already has this fallback (the doubles rally step in the sim's rally module). In order:

1. Every tiered contact search fails on this tick, as it does in play.
2. The path does not land short (its last entry has two or more bounces), so the routine goes to the "after" test.
3. The after test reads the sign of the incoming stand-spot quad's z. That quad is never initialised by the routine
   before this point; it holds the leftover scratch direction.
4. The game's leftover z on this tick is **+0.2663**, non-negative. So:
   - the partner repick and the middle repick do not happen (no repick draw);
   - the dive check runs;
   - the player chases the first entry that is falling after its bounce. That is entry 39, the run toward (about −0.41,
     −8.53), direction (0.472, −0.881).
5. That accounts for the 4 draws on 1585. On the next three ticks the game draws 6, 4 and 10.

Play takes the middle repick instead, because its leftover z on that tick is **−0.0021** (negative). Nothing else
differs: forcing play's leftover sign to non-negative on that tick (a trial, reverted) gives:
- draws 4, 6, 4, 10, matching the game;
- the run toward entry 39;
- players matching to 1588 (was 1585) and the ball to 1885 (was 1610).

So the fix is in the leftover-direction model, not in the searches, the in-time test or the run estimate.

## The leftover value

The leftover is the previous player's racket direction z (the racket chain's world x axis, z component, times the hand
sign). The rules for slot 0, the toss tick and the trail tick are unchanged from M185. Live in the European game
(2026-11-23, save slot 8, point 3's rally, rally-phase ticks):

- At the dispatcher entry, the game's leftover for slot 2 follows player 1's racket direction z. It reads −0.0021 at
  tick 409, then +0.1298, +0.2663, +0.3976, +0.5144, +0.6214 at ticks 410 to 414.
- The after test runs for slot 2 first at tick 411 (recorded 1585), then at 412, 413 and 414. It gets the same values as
  the dispatcher entry: +0.2663, +0.3976, +0.5144, +0.6214, with x −0.9580, −0.9044, −0.8324, −0.7381.
- On ticks 409 and 410 the game does not reach the fallback for slot 2, so it makes no landing-short test and no after
  test.
- Recorded tick 1585 is game tick 411. The game draws nothing on 409 and 410. Counted per frame (the frame counter
  turns over mid-tick), its draws on the four frames from 411 are 3, 7, 4, 10. The recording has 4, 6, 4, 10 on
  1585–1588: the same 24 draws, with one of 1585's draws landing in the next frame's count.

Play's racket direction model (the posed racket after the motion player runs a fixed number of extra frames) gives
exactly the same value sequence. Only the alignment differs:

- With a look-ahead of 2 frames (M185's value, proven on the US 60 Hz match), play's frame for tick 411 gives −0.0021,
  the game's value two ticks earlier.
- With a look-ahead of 4 frames, play gives +0.26627, +0.39757, +0.51441, +0.62140 on 411 to 414: the game's values to
  4 digits, x included.

The European build's look-ahead is not a fixed 4. Over the same rally phase, the game's dead stack matches play's
look-ahead-4 sequence:
- shifted 4 frames earlier (look-ahead 0) on ticks 196–255 and 555–589 (player 1's other swings);
- unshifted (look-ahead 4) on 393–423;
- 2 or 3 frames earlier on short stretches in between.

On the remaining ticks the dead stack is not player 1's racket direction. The sign only matters on ticks where the
fallback runs. Here those are 411–414 (look-ahead 3 to 5 is positive on all four; look-ahead 2 is negative only at 411; look-ahead 0 or 1 is negative at 411
and 412) and one call at 207
(negative either way).

Measured in play (trials, reverted):

| look-ahead | EU phases matched | EU players/ball part (from 1302) | US match |
|---|---|---|---|
| 0, 1, 2 | 7 of 144 | 1585 / 1610 | 2 passes (to its known part at game 2's first serve, B771) |
| 3, 4, 5 | 16 of 144 | 1588 / 1885 | 4 parts before tick 2354 |

## 50 Hz?

- The rule (fallback, after test on the leftover sign, chase the after-bounce entry) is shared by both builds.
- The value gap shows only in the European match so far. A look-ahead of 2 matches the US match as far as it is
  proven (28 phases, up to B771's part), and changing it to 4 breaks the US match before tick 2354.
- The European pose lead changes between swings (0 or 4 frames), which a single constant can't model. The cause is
  likely 50 Hz-specific: the European motion player's timing for player 1's swings, or what the European game computes
  between player 1's racket pose and slot 2's update. It is not confirmed.

## For the port

Nothing to port for B772: the fallback is already in the port and matches the game. The fix is the European leftover
racket direction, which B773 specs. With it, the leftover on point 3's tick 1585 must be +0.2663, not −0.0021. A
constant look-ahead of 3 to 5 fixes 1585, but it disagrees with the game on ticks 196–255 and 555–589, where 0 fits, so
B773 has to find the rule. After B773, the European probe (`HST_M183A1_FROM=1302`) should match 16 or more phases with
players to 1588 or later, and the US bot match should be unchanged. The next part, at 1588, is B774.

## Open questions

none for this task: what sets the European racket pose lead per swing is B773, and the 1588 dive press move is B774.
