# B775 — US bot-match probe: motion clocks from the start state

## Problem (from B773)

- The US bot-match probe (`crates/hst-play/src/play/fifty_hz.rs` tests, `us()` → `played_ticks_rng`) started player 1's
  opening idle (motion 0x0) at clip time 0.
- The recorded game's idle had already run on from the screens before the set-up.
- B773's shift fit put the doubles stand test's racket read 3 ticks off on that opening idle (play ticks 26–200).

## What the game has at the probe's start state

Start state: `fixtures/anim_s05.bin`, vsync 7565, the frame before the first serve set-up. Read with B773's
`scripts/b773_anim_show.py`.

- Player 0: serve stance (0x20), clock 2 (sampled 1).
- Players 1 and 2: idle (0x0), clock 2 (sampled 1).
- Player 3: idle, clock 52 (sampled 51). Its idle clip is longer and wraps at 54; players 1–2's wraps at 52.
- None of the four is crossfading.

## Why seeding the clock alone wasn't enough

- Play's first tick issued a fresh idle command.
- Play's motion setter (`set_motion`) restarts a player's first command even when that looping motion is already
  playing. It keeps a looping motion only once the player has a command.
- In the game, the set-up keeps those idles running. The capture shows idles carry on into the first set-up and into
  the set-up after the change of ends (vsync 14235: idle players keep their clocks, the server keeps 0x20).
- After a point-over, the set-up cuts everyone to clip 0, because they come from reactions (vsyncs 8656, 9932, 10774).

## Fix (test harness only)

- Added a `START_CLOCKS` thread-local and `start_clocks()` in the `fifty_hz.rs` test module.
- `us()` sets the four players' motion and clock from the capture. Each player gets a standing looping command for
  that motion (serial 1), so the set-up's commands keep it going.
- No gameplay code changed. The EU probe is unchanged: it doesn't set `START_CLOCKS`.

## Results

- `fifty_hz` tests pass (5). `the_us_first_serve_set_up_plays_as_recorded` passes.
- `the_us_bot_match_plays_as_recorded` (ignored test) passes with the same match result as before: 29 of 112 phases,
  point 1 has no parting, the 2354 check holds.
- Shift fit, with a temporary print of player 1's racket read (reverted; outputs in `context/b775/run2/`):
  - Every slot-2 stand read through play tick 210 (185 ticks) now fits shift 0 (was shift 3).
  - Over the whole run: 532 reads at shift 0 (was 345). Other shifts unchanged: 175 at −5, 15 at −1, 13 at −6.
  - 458 reads have no fit (B777's crossfaded motions and other scratch writers).

## Not verified / not 1:1

- Whether the game's set-up command keeps an idle that is already playing at the change of ends is seen in the capture
  but not checked in play. Play's US probe parts before game 2's set-up, so play's behaviour there is untested.
- The start values are taken from one capture (`anim_s05.bin`, slot 5 only). They are the probe's start state, not
  something play computes.
- The EU probe's opening idle is not changed. B773 found its read 1 tick off on 6 ticks. Filed as B778
  (`agent-runner plan` ID B778, after B775, files `crates/hst-play/src/play/fifty_hz.rs`).
- The server's 0x20 start is seeded too (the game is already in it). The first-serve test still passes, but its
  0x21 switch timing wasn't compared tick by tick.
