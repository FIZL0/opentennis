# P112 — unused umpire calls and unused GAME.BIN tuning (FINAL)

The audit (README-FINAL.md, *Dead code*) flagged four items. Each one checked against the original and the port:

## Umpire calls in `hst-sim/src/sound.rs` (change ends, tiebreak, match) — dead duplicates, deleted
- All three play, through `hst-sim/src/umpire.rs`: her announcement program (2), keys 3 change ends (`Umpire::start`
  with `change_ends`), 0 tiebreak (`Umpire::serve` on the first tiebreak serve, again in its instant replay), 4 match
  (`Umpire::call_match`). The match drives them from `play.rs` (change-of-ends phase, the serve set-up, match decided).
  Same program and keys as the dead `Play` constants, and `umpire_s05` / `umpire_tiebreak` already check the
  umpire's voice against recordings.
- The sound.rs constants and the umpire bank's `CALL` program constant they alone used are removed.
- `slot::JINGLES` (8): the jingles don't go through `sound::Play`; the app plays them from its own bank
  (`play.rs` `jingles`, the match-end ceremony). Removed as dead.

## `exe::Game::stick_reach` — misnamed, the value is the timing-scatter scale; deleted
- The game reads this word in one place only: the shot launch, which multiplies the timing scatter (side, depth)
  by it before turning it into the shot's frame (N5's "Scatter = 1.5·(sx, sz')"). It is never written. It is not
  how far the stick moves the aim point; the getter's doc was wrong.
- The port has that behaviour already, with the value as a literal: `serve::scatter_along` (× 1.5, used by serves,
  smashes and framed lobs), verified by the serve/smash launch tests. The disc's value is 1.5 (read from the
  user's GAME.BIN), and only SCUS-97610 US 1.00 is accepted, so the literal is exact. Not a gap; the getter is removed.

## `exe::Game::tornado_floor` — was hard-coded as 0 by the app; now read
- The game reads it in the tornado's slow-motion in-between draw only (two reads, the clamp), never writes it.
  Value on disc: 0.
- `Tornado::between` already takes a `floor`, but `hst-play/src/play/tornado.rs` passed a literal 0. It now
  reads `tornado_floor()` with `tornado_fade()` at setup and passes it. Same result (both 0), but the value now comes
  from the disc like the fade. `tests/tornado.rs tornado_slowmo` takes it from the disc too.

## Not verified / not 1:1
- `tornado_slowmo` couldn't run: its fixture `context/fixtures/tornado_slowmo.bin` isn't on this machine (not in
  this worktree or the main checkout), so the test skips. The change can't alter its result: the floor read is 0,
  the literal it replaces.
- No new PINE recording: no behaviour changed (dead code removed; a literal 0 swapped for the disc's 0).
