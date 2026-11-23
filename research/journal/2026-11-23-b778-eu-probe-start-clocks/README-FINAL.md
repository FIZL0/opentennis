# B778 — EU bot-match probe: motion clocks from the start state

## Problem (from B773 and B775)

- The EU bot-match probe (`european_rngs` in `crates/hst-play/src/play/fifty_hz.rs` tests) started every player's
  opening motion at clip 0.
- The recorded game's motions had already run on from the screens before the set-up, so play's clocks were behind.
- B773's fit had the doubles stand test's racket read off on the opening idle's first ticks.

## What the game has at the probe's start state

Start state: `fixtures/m169/eu-bot-anim.bin`, vsync 37921, the frame before the first serve set-up (first serve-phase
frame 37922). Read with B773's `scripts/b773_anim_show.py`.

- Player 0: serve stance (0x20).
- Players 1–3: idle (0x0).
- All four are at the same clock: time 23.6664 (f32 bits 0x41bd54ce), sampled 22.6664 (0x41b554ce), speed 1, no
  crossfade. The clock is fractional because the game runs at 50 Hz.
- Unlike the US start state (B775), the four clocks are equal and none has wrapped.

## Fix (test harness only)

- `european_rngs` sets B775's `START_CLOCKS` with those values. `start_clocks()` gives each player that motion at that
  clock, with a standing looping command, so the set-up keeps the idles running as the game does.
- No gameplay code changed.

## Results

- `fifty_hz` tests pass. `the_european_first_serve_set_up_plays_as_recorded` passes.
- `the_european_bot_match_plays_as_recorded` (ignored test): phase list unchanged (same 29 phase lines printed, same
  parting ticks for players at shift 1: 134).
- The held ball (server's hand, 0x20 pose) now equals the recording on recorded ticks 0–1 (before, it differed from
  tick 0). Ticks 2–7 still differ (see below). From tick 8 on it matched before and still does.
- Stand-test read fit (temporary prints of player 1's racket read in `doubles_ai/scratch.rs` and a per-tick marker,
  both reverted; B773's `b773_lead_fit.py` run against B772's run6 stack capture `context/b772/run6/eu_stack.bin`;
  outputs in `context/b778/before|after/`):
  - First rally (vsync 38050–38300): reads at shift 0 went from 86 to 98. The opening idle's rally ticks 1–6 now fit
    shift 0 (no fit before), and so do ticks 8–13, the first six fade frames of the 0x0→0x5 crossfade, which mixes
    from the idle's clock. Tick 7 (the switch tick) and the fade frames from clip 6 on still miss (B777/B779).
  - Third point's rally (vsync 39100–39692): unchanged (299 at shift 0, 6 at shift 1 in B773's numbering). Those 6
    ticks are that rally's idle, which restarts at the point's set-up, so the start state can't reach them. B773 had
    attributed them to the probe's opening idle; that was wrong. The opening idle's mismatch was in the first rally,
    which B773's fit didn't cover.
  - In the third rally the game's idle is at clip 4.33 on rally tick 1, while play's is at 1.33–2.33. Play's idle
    there wraps a few ticks after the game's, yet the read fits only 1 tick off. Not followed up here.

## Not verified / not 1:1

- The held ball on recorded ticks 2–7 (the serve set-up's 0x20→0x21 crossfade, old clip from 26.67) is off by up to
  about 0.3 in height (e.g. tick 2: game y −0.522, play −0.825). The US probe has the same window (its first-serve
  test starts the held-ball check at tick 9). Filed as B780 (after B778, files `crates/hst-play/src/play.rs`,
  `crates/hst-play/src/play/fifty_hz.rs`).
- The third rally's 1-tick shift on the idle (above): not explained; it isn't the start state.
- The start values come from one capture. They are the probe's start state, not something play computes.
