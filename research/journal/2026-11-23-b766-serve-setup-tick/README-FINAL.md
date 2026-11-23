# B766: the serve set-up a tick long at 30-40 and deuce was the recording's count

## What was going on

The task expected a score-dependent wait in the game: in slot 5's bot match (`fixtures/match_s05.bin`) the serve
phases of points 6 and 8 looked a tick longer than in play (180 vs 179, 154 vs 153), with the ball unplaced at
(0,-1.064,0) for two ticks instead of one.

There is no such wait. The game's own phase tick counter (gm+0x58) ends those phases at 179 and 153, the same as
play. What differs is the first sample of each phase. At most set-ups the recorder's first frame of the new serve
phase reads counter 1 (one tick ran during the load stall). At some it reads 0, sampled before the first tick had
run. The next sample reads 2 in both cases.

The test harness's `recorded_ticks` (`crates/hst-play/src/play/fifty_hz.rs`) pushed max(1, counter) copies of a
phase's first sample. A counter-0 start therefore became a phantom extra tick, and everything after it in the
recording sat a tick late (the "players part at 4011").

In `match_s05` the counter-0 starts are the serve set-ups at 30-30, 30-40 and advantage in game 1 (the task's
"after 30-40 / deuce" counted the points one off), and later ones in games 2-4 (e.g. 15-0, 30-30, 15-15, advantage).
The EU recording (`eu-bot-match.bin`) has none, so its tests are unchanged. No decompile read was needed, so there
is no spec phase: the task changed only the test harness.

## Fix

- `fifty_hz.rs` `recorded_ticks`: at a phase change it pushes `counter` copies of the new phase's first sample (none
  when the counter is 0). The next sample's count then repeats it for tick 1.
- `the_us_bot_match_plays_as_recorded` now asserts that 25 phases match (was 12: the whole first game and the change
  of ends), and that every player matches from the first serve to tick 6845 (was 4011). The exceptions are each
  point-over's last tick (1084, 2354, 3190, 3795, 4391, 5011, 5689, 6543): the known one-tick placement offset.

## Where it parts now

Game 2's first serve. The game's serve phase lasts 188 ticks, then a 49-tick rally and an 80-tick point-over (the
short ones a fault takes). Play's serve lasts 195 ticks and the rally 380. Players first part at recorded tick 6845.
Filed as a follow-up: B767.

For M183a1: B766 no longer blocks it. The US match now parts at game 2's first serve.

## Not verified / not 1:1

- Nothing for this task itself: play's serve set-up was already right, and no game code changed.
- Game 2's first serve gap is left to the follow-up.
