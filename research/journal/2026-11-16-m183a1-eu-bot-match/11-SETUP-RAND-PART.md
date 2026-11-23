# Part 11: the European match's setup rand() draws (PART: blocked on B782)

## Finding
- Play's European rand() stood a constant 1578 draws ahead of the game at every point's shared reseed. The recorded
  seeds are the rand() outputs 89805, 103535, 112706, 137645…; play's were 91383, 105113, 114284.
- So the shared generator never matched in the EU match. That is why point-over reactions (they draw from it) parted
  at 465 and 1886, while the AI generator matched every tick.
- A rand() trace from `eu-bot-start.p2s` shows the game's draws:
  - 81923 after the lens-flare setup.
  - +1 at vsync 37360 (sound reseed).
  - +30 at 37518 and +2 at 37521 (81956).
  - Then exactly 327 intro flare ticks of 24 draws each (to 89804).
  - Then the shared seed at output 89805.
- The trace came from the new script `scripts/eu_rand_rec.py`. It logs vsync, rand state, phase, sub-phase and intro
  tick each frame, and retries the state load until vsync ≥ 30000. It ran on PCSX2 copy 2 with the current PCSX2
  build, since v2.9.52 refuses that state.
- Play runs 394 intro ticks (court 6: camera 150+150, banner 90, +2, the disc's 60 Hz count) and no setup clouds in
  doubles. The gap is fully explained: 1578 = 67 × 24 − 30.
- Play already makes the +1 (sound reseed) and the +2 at vsync 37521 in its own setup, so only the 30 cloud draws and
  the 67 extra intro ticks differ. Checked empirically: with 6 setup clouds and 327 intro ticks, play's shared seeds
  land exactly on outputs 89805, 103535 and 112706.
- The 327 intro ticks run 1–125, 126–250, 251–252 and 253–327 (125 + 125 + 2 + 75). The camera moves and the banner
  come at 5/6, with only one of the two +2s.
- The 30 draws are 6 clouds × 5, court 6's doubles cloud count. So this setup spawned clouds in doubles.
- P3f3 saw no clouds in two US court 10 doubles logs and put it down to an uninitialised byte compare. Slot 5's US
  offset (2642) also fits 50 setup draws.
- The real condition is not known.

## Harness (tests only, plus one call site)
- `fifty_hz::intro_flare_ticks(iso, iso_path, stage)` returns the intro's flare-tick count. `play.rs`'s setup calls it
  instead of `Rngs::intro_ticks_on_disc` directly.
- Under test a probe can set `INTRO_TICKS` (disc path and count). It is process-wide, because Bevy runs setup on its
  own threads. The EU probes already take turns on `EU_DISC`.
- `played_ticks_rng` takes the setup's cloud count from a test-local `SETUP_CLOUDS` (default 0). The EU probe sets
  6 clouds and 327 intro ticks.

## Result
- The shared generator is now seeded as recorded (89805, 103535, 112706). `the_european_generators_play_as_recorded`
  passes.
- 20 of 144 phases match (16 before).
- Point 1's players now part at 571 (465 before).
- From 1302 a player parts at 1992 (1886 before), with the phase at 1858.
- Point 4's rally (phase 20) runs 445 ticks against 398 recorded. Not looked into.

## Follow-up
- B782 (spec): the 50 Hz intro length and when the setup spawns clouds. The port replaces the test hooks.

## Status
Blocked on B782. Then: the remaining parts (players at 571, phase 20), EU tick for tick, flip
`match_speed::FIFTY_HZ_SIM`, and add the release note.

## Not verified / not 1:1
- The 327 ticks and the 6 setup clouds are the recording's numbers, put in by the probe. They are not a rule. Other
  courts, singles and other heap states aren't covered (B782).
- The parts at 571 and phase 20 weren't diagnosed.
- `FIFTY_HZ_SIM` isn't flipped and there is no release note.
- The earlier parts' items stand.
