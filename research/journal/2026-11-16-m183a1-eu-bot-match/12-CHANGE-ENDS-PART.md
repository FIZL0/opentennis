# M183a1 part 12: after B782 (PART: blocked on B784 and B785)

Reopened after B782 landed. Probe:
`HST_M183A1_PHASES=200 tools/check.sh -p hst-play the_european_bot_match_plays_as_recorded -- --ignored --nocapture`.

## Baseline
- 20 of 144 phases match after B782.
- Point 1's tick-571 part is a torn-sample artefact: the phase lengths match, and the ball repeats at stall ticks.

## Phase 20: a lob that should be flat
- Phase 20's rally matched tick for tick from 5841 to 6152.
- At 6152 player 1's shot came out a lob in play (spin 8.744). In the game it is flat.
- The press (✕), the contact tick, the kind at contact (flat) and the AI draws (73) all match. So it is a mis-hit roll from the shared generator.
- The shared generator stopped matching at 5615, game 2's first point.

## The point seeds
- New ignored test `eu_point_reseeds` (in `fifty_hz.rs`) searches the rand() stream for the seed of each point's recorded shared generator. It needs `fixtures/m183a1/eu-rng-long.bin`.
- Every point sits at offset −3, except 5615 (offset −1), the first point after the change of ends.
- A recording of the game's rand() state counts the draws. It is 7200 frames from scratch slot 8, via `scripts/eu_rand_rec.py`.
  - A normal point start takes 3 draws besides the flare: shared, court and sound reseeds.
  - The change of ends' start takes 26: 24 for the flare, plus 2.
  - Its end takes 24 plus 2 (shared, court, no sound).
- Play draws only 1 at the start (the sound reseed). The source of the missing draw needs the original: B784 (spec).
- Test-only stand-in: `fifty_hz::CHANGE_ENDS_DRAWS`, set from `HST_M183A1_CHANGE_ENDS=n` inside `european_rngs`. It makes play draw n more rand() as a change of ends starts.
  - It lives in `play.rs` under `#[cfg(test)]`, defaults to 0, so nothing changes otherwise.
  - Whether the draw comes before or after the sound reseed makes no difference to the probe.
  - B784's port replaces it.
- With `HST_M183A1_CHANGE_ENDS=1`: 24 of 144 phases match, the first 24 in a row. The 5615 seed lines up.

## Phase 24: a net-cord fault
- Phase 24 is the point-over after a serve fault whose ball clipped the net.
- It records 64 ticks, and play plays 65.
- The other fault (phase 33, no net cord) matches.
- The call timing at 50 Hz for that case needs the original: B785 (spec).
- Past that, phases match again to 37.
- Phase 38's rally runs 92 recorded ticks against 138 in play. Phases 40 onwards diverge. Not looked at; this likely follows from the generators parting after phase 24.
- Positional parts have matching lengths at phases 11, 14 and 21 (the point-over players' spots). Not looked at.

## Code
- `fifty_hz.rs`: the EU test also runs `shot_probe`. `HST_M183A1_PHASES` sets how many phases it lists. It also holds the stand-in and `eu_point_reseeds`.
- `play.rs`: the test-only stand-in loop, after the change of ends' sound reseed.

## Status
Blocked on B784 and B785. When they land: EU tick for tick, then flip `match_speed::FIFTY_HZ_SIM`, and add the release note.

## Not verified / not 1:1
- Whole-match proof not done. EU parts at phase 24 (tick 6636).
- The extra change-of-ends draw is a stand-in with an unknown source, for tests only.
- Phases 11, 14 and 21 (positional parts) and phase 38 onwards are not diagnosed.
- `FIFTY_HZ_SIM` is not flipped and there is no release note.
- The earlier parts' items stand.
