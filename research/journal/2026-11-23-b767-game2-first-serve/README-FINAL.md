# B767: game 2's first serve parted because play's computer players kept drawing between ends

## What was going on

In slot 5's bot match (`fixtures/match_s05.bin`) the game's first serve of game 2 had its set-up at 188 ticks, then a
49-tick rally and an 80-tick point-over: a fault. Play's set-up lasted 195 ticks and its rally 380, so the serve
landed in. The players first parted at recorded tick 6845.

## Finding

The ball's bounces before the toss match bit for bit, but the game's server starts its toss 16 ticks earlier. The
toss then flies the same curve bit for bit, just 16 ticks apart. So the server's random wait before the toss
(60..119 frames, one AI generator draw) differed: the AI generator had drifted.

The long generator recording (`fixtures/m183a1/us-rng-long.bin`) ends at vsync 12719, before game 2. A new recording
of slot 5's generators over vsyncs 12551..14550 was taken, covering the change of ends (vsync 14154) and game 2's first
serve set-up (vsync 14235). It used `scripts/us_rng_window.py` (the M183a1 `us_rng_rec.py`, with the wait for the
start vsync no longer capped at 600 frames):

    tools/pcsx2.sh python3 research/journal/2026-11-23-b767-game2-first-serve/scripts/us_rng_window.py 5 2000 <out> 12550

2000 samples, no gaps. It was copied to `fixtures/b767/us-rng-game2.bin` (git-ignored) for the test.

Play's AI generator stayed on the recording through point 10's point-over (tick 6543) and left it for good at tick
6544, the change of ends' first tick.

In the game, the change of ends' first frame makes 87 AI draws and then none for the rest of the phase: the point
reset (17 per player, 68), the server's serve-state entry (17), and one draw each for the two players entering the
rally state. The receiver's entry draws nothing. That is the same set of draws as a normal serve set-up's start.

Play drew 92: the same 87, plus the receiver's stand-in walk roll on entry, the server's serve routine picking its
spot and wait (1 draw), and the stand-in walk's 30-tick re-rolls (2 draws at tick 31 of the phase, 1 at tick 61).
Five extra draws, so every AI draw after it was shifted, starting with game 2's serve wait.

## Fix

- `crates/hst-play/src/play.rs` `ai_between_ends` and `crates/hst-play/src/play/doubles_ai.rs` `enter_between_ends`:
  between ends a computer player makes only its AI state's entry (the receive's or rally's entry; the serve's entry is
  already in the AI update) and stands. No serve routine, walk or re-roll runs until the next point.

No decompile was read for this: the rule comes from the recorded generator only, so there is no spec phase.

## Result

- `the_us_bot_match_plays_as_recorded` now asserts that 27 phases match (was 25: game 2's first serve set-up at 188
  ticks and its 49-tick rally ending in the fault). Every player matches to tick 6899 (was 6845). Each point-over's
  last tick is as before.
- With `fixtures/b767/us-rng-game2.bin` present, play's AI generator is on the recording's draws for every tick
  6500..6812, except 6544 and 6625 (the two phase changes' ticks, as at every point).

## Where it parts now

The fault's point-over is 80 ticks in the game and 38 in play (play's second serve set-up starts at tick 6900).

Around the serve's press and contact, the game's AI draws come as 1, 69, 2, 1 (ticks about 6811, 6813, 6839, 6844).
Play's come as 1, 2, 66, 2, 1 (6813, 6814, 6815, 6840, 6846). Play is one draw short before the fault call's 4 draws
(game 137 vs play 136 at the call), and the call's draws land 2 ticks later in play (6863 vs about 6861).

Filed as a follow-up: B769.

## Not verified / not 1:1

- Singles change of ends: there is no singles recording over a change of ends with its generators, so the same rule
  is assumed for singles. The European 50 Hz bot match tests still pass.
- The fault's point-over length and the serve-contact draw order are left to B769.
- Whether the game's change-ends draws all come in one frame while play spreads them over two ticks (reset tick,
  entry tick) isn't told apart. The same split already matches at every serve set-up's start.
