# M181: EU 50 Hz port (sim switches) — FINAL

Port phase of M169a's spec (`research/journal/2026-11-16-m169-eu-50hz/SPEC.md`), decompile locked. The rules come
from the spec; every constant below is checked against M177's European bot-match recordings (`fixtures/m169/`).

## What landed

- `hst_sim::build::Build::Europe`: the US rules (`us()` is true for it) plus the 50 Hz switches: `gravity`,
  `toss_gravity`, `run`, `soft_speed2`, `serve_walk`, `toss_release` (45 → 37.5), `rest_timeout` (1800 → 1500),
  `serve_wait` (`%50+50`), `frames` (29 → 24, else ×5/6), `motion_rate` (80 → 96 ticks a frame, except motions 16–31,
  37, 38 and the dummies 48, 49, 51), `clip_length` (the ×1.0000010 nudge on lengths that aren't whole), and km/h
  at 50 per second (`kmh`, `ball_kmh`).
- Build variants beside the US functions, so 60 Hz goes through the same code: `ball::Flight.build` (gravity, rest
  timeout), `serve::toss_velocity_in`, `player::run_speed_in`, `player::serve_walk_in`, `motion::soft_follow_in`,
  `ai::serve_wait_in`, `pose::Clip::for_build`, `hst_data::ani::Anim::retimed`.
- `hst_data::exe::Game::shot_params_at(hz)`: at 50 the European table is read raw, with no ×1/1.2.
- `build_rules::build_at`: European disc + 50 Hz → `Build::Europe`. It is gated by `match_speed::FIFTY_HZ_SIM`
  (still off), so players see no change yet.

## Verified bit-exact (`crates/hst-sim/tests/europe.rs`)

- Live ball over the whole European bot match: 19371 frames, 361 contacts. With `Build::Us`, every frame diverges.
- Tosses: 47 bit-exact at toss gravity bits 3b673604. The neighbour 3b673605 fails, and so does the US value.
- CPU serve walk: 2224 steps at bits 3d23d69f. With US they're off on every step.
- Run velocity: 11851 frames at bits 3db03740, from each player's stats in the European RAM. With US they're off.
- Clip lengths: 67 (character, motion) pairs, 29 of them at the US rate (swings and serve swings).
- `tools/check.sh -p hst-sim`: all 325 tests pass, so the US replays are unchanged.

## Spec open questions

1. Settled by measurement, not yet ported (M182). Measured in game ticks, the European point-over phase is shorter
   than the US one:

   | Phase | US (match_s05) | Europe | Difference |
   |---|---|---|---|
   | Point | 149 | 134 | 15 |
   | Fault or let | 80 | 65 | 15 |
   | Game, before change ends | 280 | 255 | 25 |
   | Change ends | 81 | 67 | 14 |

   So the European flow's own waits are retimed. The next point does not wait on an unretimed table.
2. The ball replays bit-exact with no wind term over the whole match on court 6, so wind doesn't touch the ball
   there. A windy court wasn't recorded.
3. Not settled. The overlay's odd-ratio list (6.4 → 6.34, 8.7 → 8.58) has no reader found from the recordings. It's
   treated as presentation.
4. Settled for player motions 0–47 of characters 0 and 1: swings 16–31 and serve swings 37/38 keep the US rate;
   everything else is rescaled and nudged.

## Not verified / not 1:1

- Nothing calls the new variants in play yet. Wiring them in play.rs, raw EU TRAJ and SAITOU/TRAJ (no M170 regen,
  no M174 lending), and `shot_params_at(50)` are follow-up M183. So is the full 50 Hz bot-match proof, which is
  why `FIFTY_HZ_SIM` stays off.
- Raw EU TRAJ / SAITOU/TRAJ are not read at 50 Hz yet: `iso.rs` was not changed (M183).
- The CPU serve wait `%50+50` has no European AI-object recording to test against. It comes from the spec only.
- `Build::frames` (AI waits 29→24, 60→50, 30→25, the run counter, the path buffers and the 1800 limits) is not
  applied anywhere yet and not verified (M183).
- Soft follow-through threshold (bits 3e9e0653): from the spec. No test picks out the soft-follow frames.
- Dummy motions 48–51 aren't in the motion player's range in this capture, so their rate is unchecked.
- Match flow timing (open question 1) is measured but not ported (M182).
- Wind and the overlay list (open questions 2 and 3) are presentation. Only court 6 was recorded.
- Only characters 0 and 1 and court 6 appear in the European recording.
