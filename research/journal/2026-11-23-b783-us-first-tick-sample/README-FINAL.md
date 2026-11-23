# B783 — US probe's tick 0 held ball vs the stance frame it ends on

## Problem (from B780)

- US bot-match probe (`crates/hst-play/src/play/fifty_hz.rs`, `the_us_first_serve_set_up_plays_as_recorded`): on
  recorded tick 0 (the first serve set-up tick, server in the 0x20 serve stance) the held ball was one stance frame
  ahead of the recording.
  - game [2.9440882, -0.5307655, -12.321224], play [2.9468277, -0.5111898, -12.317473].
- Seeding the stance at clock 1 / sampled 0 matched tick 0 but made tick 1 lag. So the held check started at tick 1.

## Cause: the recording's tick-0 sample was taken mid-tick

- `fixtures/match_s05.bin`: the tick-0 sample (vsync 7566, phase counter 1) has the ball bit for bit the same as the
  frame before the set-up (vsync 7565). The stance moves the hand every frame, so the ball wasn't placed yet.
- `fixtures/anim_s05.bin`, read with B773's `research/journal/2026-11-23-b773-eu-racket-lead/scripts/b773_anim_show.py`:
  - at vsync 7566 every player's motion clock and crossfade countdown are still vsync 7565's (server: sampled 1,
    time 2, countdown -99; players 1–3 the same);
  - at vsync 7567 (counter 2) they are two steps on (sampled 3, time 4, countdown -101).
  - So the game steps once a tick. The recorder sampled the tick that runs after the loading stall, after its counter
    had moved but before its motion step and ball placement ran.
- Not just this set-up. In `anim_s05.bin`, several set-ups' first counter-1 sample shows the new motion set but not
  stepped (vsyncs 8656, 9932, 10774, 12613: time 0, countdown 7, next sample two steps on). Others show the step
  already done (14235, 14555, 15730).
- A fresh capture from slot 5 (`tools/record_live.py` and `tools/record_anim.py`, 90 frames each, in `context/b783/run1/`)
  gives the same result. Deterministic, not a one-off.
- So play's tick 0 (the stance's third frame, clock 2→3) is right. The recording never sampled the game's finished
  tick 0. No gameplay change.

## Check

- The stance frame tick 0 ends on comes round again at recorded tick 11. The server restarts its stance 0.2333 to the
  left at tick 9 (x 3.0 → 2.7666686).
- Ticks 9–11 hold the ball where the stance's first three frames did (vsyncs 7564, 7565 and tick 0's frame), with the
  same y/z bits and x shifted. Play already matches ticks 9–11 bit for bit.
- Play's tick-0 ball minus the server's x equals recorded tick 11's ball minus the server's x, bit for bit
  (-0.05317235). y and z have the same bits.
- Test harness only: new `us_first_tick` in the `fifty_hz.rs` tests:
  - asserts the recorded tick 0 is the stale ball from the frame before the set-up;
  - asserts tick 10's y/z equal that frame's;
  - replaces tick 0's expected ball with tick 11's, moved back by the server's step.
- `the_us_first_serve_set_up_plays_as_recorded` now runs the held check (`serve_to_contact`) from tick 0.
  Result: bit-exact on ticks 0–168.
- `tools/check.sh -p hst-play fifty_hz`: 28 tests pass.

## Not verified / not 1:1

- The game's finished tick-0 ball is never sampled directly. The recorder samples at vsync, and that tick spans the
  stall. Tick 0's expected value is rebuilt from tick 11's same stance frame (y/z copied, x shifted by the server's
  step, which round-trips exactly in f32). It's a check of the pose and placement, not a direct read of that tick.
- Other set-ups' torn first samples (listed above) only touch the ignored `the_us_bot_match_plays_as_recorded` probe.
  Its asserted spans weren't changed.
