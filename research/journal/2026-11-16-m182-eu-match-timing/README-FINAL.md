# M182: EU 50 Hz match timing (point-over and change ends): FINAL

Settles M169a spec open question 1. The European point-over phase runs on its own retimed flow, not on the US
scoreboard table: that table (waits 20/90, holds 1.0/2.0 s, rise 6, drop 10, call countdowns) reads the same from
the European GAME.BIN at its shifted offsets (`exe::scoreboard_timing` already handled the shift; no change needed).

## What changed in the European flow (all fitted against `fixtures/m169/eu-bot-match.bin`)

- Scoreboard pause 30 → 25 ticks (`Build::frames`, the spec's "more than 25"). The call-chained pause of 1 stays.
- A score's hold is its seconds × 50 ticks, not × 60: a point loses 10 ticks, a game or set 20.
- Together: point 149 → 134 (−15) and game 280 → 255 (−25), as measured.
- Change ends 80 → 66 (`flow::change_ends`): 81 → 67 phase ticks. This matches the spec's camera "cut countdown 80 → 66".
- Call sprite: hold 29 and fade 5 stay the US ticks. A fault or let is 15 ticks shorter (80 → 65) only because her
  voice line ends sooner in ticks. The recording's umpire is umpire 0, voice a: the US menu bytes, at the
  European boot-data shift 0x14d00 in `eu-bot-start_ee.bin`. Her fault and out-after-net clips are both 29 frames
  of 60 Hz audio, the same on both discs. At 50 Hz that is 29·50/60 = 24 ticks, and the line is heard ending
  `VOICE_LAG` (5) ticks later, a tick sooner once. That gives line end 29 (10 phases) and 28 (1 phase): the US rule
  exactly. A retimed hold (24/4) would need an 11-tick lag, so it was rejected.

Port: `PostPoint::in_build(Build)` (the pause and the hold's per-second factor), `flow::change_ends(Build)`; the US
path is unchanged (`Build::Us` default, `#[serde(default)]`).

## Proof

`crates/hst-sim/tests/europe.rs::match_flow_runs_on_the_european_timing`:

- 47 point-over phases asked for the next phase on the game's own tick, with faults, points and games matching
  every tick. 11 of them were called, with line ends checked against umpire 0's clips × 50/60 + lag.
- 3 change-ends phases last 67 ticks.
- The US flow fails on more than half of the phases.

`tools/check.sh -p hst-sim`: 326 pass. That includes `match_s05_post_point`, so the US replays are unchanged.

## Not verified / not 1:1

- The call sprite's hold and fade are inferred: they can't be told apart from the voice-line end in this
  recording. The US values, together with the 50/60 voice length and the unchanged lag, are the only consistent
  reading, but no recording shows the sprite's fade directly.
- Umpire 0 only. The recording has no out, double fault or let calls (only fault and out after net), so those
  call types' line ends are unchecked at 50 Hz.
- No human press, tiebreak, set or match-over phase in the European recording, so the set's 2.0 s hold at × 50
  is covered only through games.
- Not wired into play yet. `PostPoint::in_build`, `change_ends` and the 50 Hz voice-line ticks go with M183
  (noted there). `FIFTY_HZ_SIM` stays off, so players see no change.
