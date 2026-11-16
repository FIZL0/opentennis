# M183: EU 50 Hz in play

Port phase from `research/journal/2026-11-16-m169-eu-50hz/SPEC.md` (M169a) on top of the sim switches (M181) and the
match flow (M182). `match_speed::FIFTY_HZ_SIM` stays **off**, so the US/JP/EU-at-60 game doesn't change; the switch
flips in M183a, after the bit-exact proof.

## What play does at 50 Hz now

- **Which program**: `build_rules::of` picks the build at `match_speed::sim_hz()`. That is 60 while `FIFTY_HZ_SIM` is
  off, unless a test forces it with `test_sim_hz`. `build_rules::apply` also puts the build on the ball flights
  (`Flight.build`, which `relaunch` carries over), every player's `Body.build` and their AI rows (`AiParams.build`).
- **Sim switches asked with `g.build`**: serve walk (`serve_walk_in`, `Build::serve_walk`), toss release and toss
  gravity (`toss_release`, `toss_velocity_in(.., toss_gravity())`), approach run speed and stamina drain
  (`run_speed_in`, `drain_in`), soft follow (`soft_follow_in`), the bot's serve wait (`serve_wait_in`), and the shot
  table (`shot_params_at(build.hz())`).
- **The sim's own stamina counter**: `player::drain_in`, `Runner::frames_to_in` and `Body::step` in rally.rs use
  `build.frames(STAMINA_FRAMES)` (60 → 50). The AI's km/h timing test uses `build.hz()`.
- **AI waits**: the singles repick (`ai_wait` in play.rs, rally.rs's team repick: more than 29 → more than 24), the
  give-way compare (< 30 → < 25) and the return wait (≥ 60 → ≥ 50), all through `Build::frames`.
- **Data**: `iso::set_own_rate` is set from `match_speed::set_current`, or from `test_sim_hz`. It turns off the M174
  lend overlay and the M170 TRAJ regeneration, so the European disc's own TRAJ and SAITOU/TRAJ are read as they are.
- **Motions**: `fifty_hz::retime` runs on every character the match loads (disc characters before they are cached,
  mod characters through `Arc::get_mut`). It moves each clip to `Build::motion_rate` for its motion number (0..47 by
  number, the team reactions and poses after them as no player motion) with `Build::clip_length`. It also moves the
  visual takes, the root paths and the stance ball track. The toss hand is sampled from the clip at its file rate
  (`fifty_hz::file_rate`): frame 45 at 80 tpf is tick 3600, the same tick as the 50 Hz release (37.5 × 96).
- **Flow**: both `PostPoint::called` sites use `.in_build(g.build)`. Every change-ends length (play.rs, change_ends.rs,
  own_camera.rs, popups.rs) is `flow::change_ends(g.build)` (66). The weather fade weight scales its tick to the
  80-tick span. The umpire's call line lasts `frames × 50 / 60 + VOICE_LAG` ticks (`fifty_hz::voice_ticks`).

## Test

`play::fifty_hz::tests::a_bot_match_at_50_hz_plays_the_european_program` uses `fixtures/discs/SCES-54535.iso`, chars
0/1, court 1, at a forced 50 Hz. It checks:

- `Build::Europe` is on the game, the flights, the bodies and the AI.
- Every player motion that character 0 loads equals `Clip::for_build(.., Build::Europe, Some(id))` from the disc's own
  file, by tpf and by length bits. Some clips keep 80 tpf and the rest go to 96.
- TRAJ is the disc's own bytes (`read == read_disc`).
- In 150 s of play: 10 serves, all tossed, rallies up to 17 shots, and the change of ends lasts 66 ticks.

The two other European-disc tests in build_rules.rs share the `fifty_hz::EU_DISC` lock with it, because the forced rate
is process-wide. hst-sim (780), hst-play (260), hst-data iso and hst-base match_speed all pass, the US replays included.

## Not verified / not 1:1

- **The whole 50 Hz bot match is not proven bit-exact** against `fixtures/m169/eu-bot-match.bin`. The test only shows
  it plays on the European timing. Moved to M183a, which also flips `FIFTY_HZ_SIM`.
- **The path history and point/replay gates are not ported yet**. That covers the 2246-word buffer, the countdown of 25,
  the long limit of 1500, and the gates 900 → 750 and 1800 → 1500 (`instant.rs`, `instant_replay.rs`'s `LONG_RALLY`).
  Moved to M183a.
- **The motion-number mapping is inferred**. `retime` treats keys ≥ 48 (team reactions, poses) and the root paths and
  stance ball track as following their clip's motion number, or as no player motion. The spec covers player motions
  0..47, and only those are compared against `Clip::for_build`.
- **Visual and ceremony clips** (the visual takes, the change-ends and point-over poses) are retimed with the same rule.
  No recording checks them. Presentation only needs to feel right.
- **Presentation timing**: the change-ends camera, pop-ups and weather fade follow the 66-tick phase. These are by-eye
  presentation, with no capture compared. The umpire line length rounds down at `frames × 50 / 60`, and the round-down
  is an assumption.
- **Coverage**: the European recording covers characters 0/1 on one court only. Other characters' motions are retimed
  by the same rule, and none of them is compared against the game.
