# B720: how long a dive keeps the routines' stroke flag

Done from recordings only; the decompile was not read.

## What the game does

The routines' stroke flag stays set from a dive's first tick until **5 ticks after the dive's lock ends** (the
lock is the slide frame + 51 ticks, already proven against the dive counter in `hst-sim/tests/swing.rs`). Slot 5's
US dive (server's partner, pressed at recorded 319, a 15-frame dive that falls short): flag set 320–390, 71 ticks;
play's dive lock is 66 (320–385). The game's body also shows the swing countdown at −3 for the first 21 ticks, then
−1 for exactly 50 ticks (the ground recovery), and `moving 2` all through; both clear on the same tick as the flag.

Other dives in the fixtures (probe `b720_dive_lengths`, ai_rally.rs, ignored; `b720_dive_flags` dumps one
recording tick by tick):
- `ai_net_s05` / `ai_rally_s05_all`, the same US dive: −3 ×21, then the flag clears 71 ticks on.
- `ai_rally_s05_all` 0x1074470 frame 256: −3 ×11, then −1 ×50, so 61 ticks. That rules out a fixed 71; it fits
  lock + 5 if that dive's slide was 5 frames.
- `ai_rally_singles` (0xc56ad0 at 305 and 975): −3 ×21, then 71 ticks. Singles behaves the same.
- Runs of −3 for 5 or 9 ticks followed by 30 (not 50) are some other stroke, not dives. I did not look into them.

## Change

`doubles_ai`: each computer player keeps a 5-tick hold (`DIVE_HOLD`), reset each tick its dive is live and counted
down in its step. Its body's stroke flag stays set while that hold lasts. Locomotion still follows the dive lock, so
positions don't change.

## Result

- US probe: the players now part at 855 (was 417). The ball matches the recording through the whole first point
  (was 703). The AI generator matches every tick through 869 except the known 218 (was 386).
- `the_us_bot_match_plays_as_recorded` now asserts the generator through 869 and the players' part at 855, and
  checks the ball through the point.
- EU probe unchanged (its first gap, at 303, comes before any dive).
- Follow-up **B722**: player 1 leaves at 855 (0.005 m, then 0.14 m) while the ball stays exact. The generator
  leaves at 870.

## Not verified / not 1:1

- The rule "lock + 5" rests on two dive lengths: the US dive (15-frame slide, known from play) and one more whose
  slide length I inferred from its −3 run, not read. A fixed −3 run of 21 for every dive is ruled out by the
  61-tick case, but I did not check a dive whose slide length is recorded alongside its flag.
- The game's body reports `moving 2` during the dive and the hold, but play reports 0. No decision in these matches
  read it (the generator stays on the recording), so I left it.
- The swing countdown during a dive reads −3 in the game but −1 in play. Not checked whether any routine reads it.
- The 5-tick hold isn't cleared at a point's end. A dive within 5 ticks of a new point's first AI step would carry
  it over. The recordings have no such case.
