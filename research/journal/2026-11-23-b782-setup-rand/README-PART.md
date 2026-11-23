# B782 — the setup's cloud draws and the 50 Hz intro length (spec)

- Task: from the original, find (1) the intro's tick count at 50 Hz and (2) when the match setup spawns clouds. Use the findings to replace the European bot-match probe's `SETUP_CLOUDS` and `INTRO_TICKS` test hooks (M183a1, part 11).
- Outcome: `SPEC.md` written. Port phase next.

## What was done

- Read the US decompile and the European decompile (`context/m169/decomp/eu_game.c`, M169's build).
  - The setup, cloud respawn, weather-set and intro-phase routines are exact matches between the two programs, so no change there.
  - The 50 Hz program's only intro difference: its camera-table loader scales each non-zero record's frame count by `trunc(frames × 0.8333333)` at load (the f32 constant, multiplied with the EE's chop: 150 → 124, where an exact 5/6 would give 125). M169's classify lists this as "camera table / camera-edit durations ×5/6".
- New script `scripts/b782_intro_watch.py`. From a save-state load, it logs per frame:
  - match tick and phase
  - the two intro presentation gates (camera shot done, banner done)
  - the `rand()` state
  - the court's first weather, its setup spawn flag, and the weather holder's remembered-weather byte
  - It works on EU (SCES-54535) and US.
- Run 1 (EU disc, `fixtures/m169/eu-bot-start.p2s` as slot 8; output `context/b782/run1/watch.txt`):
  - EU: camera done at intro tick 250, banner done at 326, phase 2 at 327.
  - US equivalents (P3f6): 302, 393, 394.
  - The holder's byte read 8 before setup. The first weather was 0 and the spawn flag was 0 (doubles). At vsync 37518 (the +30 draws) the byte became 0.
- New script `scripts/b782_held_source.py`, run 3 (`context/b782/run3/held.txt`). It dumped the holder's memory before and after allocation. The remembered-weather word holds the same stale pointer before and after allocation, left in a reused heap block (low byte 8). So the condition comes from heap history, not from court, weather or build.
- Run 2: US slot 5 starts mid-intro (tick 60), so it can't show a US setup's byte.
- Computed every court's 50 Hz intro length from the disc's camera tables (table in `SPEC.md`).
  - Court 6: truncating multiply gives 124 + 2 + 124 + 75 + 2 = 327, which matches the recording. Round-to-nearest would give 329.
  - M183a1 part 11's split of 125 + 125 + 2 + 75 was wrong: both +2s remain.
- Netplay: `seed_match` (the test harness in `netplay.rs`) takes the setup's cloud count from its caller. The spec requires it and `main.rs` to use one function with the same default.

## Not verified / not 1:1
- Why P3f3's two US doubles setups had no remake (their stale byte equalled the first weather) isn't explained beyond heap history; no US setup was traced here.

- The 50 Hz intro lengths for courts other than 6 come from the rule, not from recordings.
- The banner's 75 frames at 50 Hz are measured (76 ticks after the camera). That they come from the animation loader's 6/5 ticks-per-frame was not traced for the banner's own load.
- The remembered-weather byte's value at a menu-started match can't be derived (heap garbage). The spec's default (none means always remake) rests on the EU bot match and the P3f 100-draw singles menu logs. P3f3's two US doubles logs show no remake. US setups were not re-recorded.
- Whether any draw falls between a singles setup spawn and its remake was not recorded (no singles setup trace).
- Existing US tests that assumed no doubles remake (for example US slot 5's offset, which M183a1 noted fits 50 setup draws) are for the port phase to reconcile.

## Next

- Port phase (decompile locked): implement `SPEC.md` in `rng.rs`, `main.rs`, `play.rs` and `fifty_hz.rs`. Replace `SETUP_CLOUDS` and `INTRO_TICKS`. Make netplay `seed_match` use the same rule. Test `fifty_hz`.
