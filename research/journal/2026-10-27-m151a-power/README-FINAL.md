# M151a: generated trajectory tables, §6 `shots` `power`

## What landed
- §6 `shots` entries can now be objects, `{"from": N, "power": x}`, with either key optional (no `from` means
  the donor's shots). `power` runs 0.5–2 and is stored in `Build::power` (per class, 1 = stock). Bad keys, a
  power out of range or a non-number are rejected (`mods::tests::bigger_disc_body`). A mod with a power ≠ 1 is
  refused online because `Build` differs from the default. Carrying it on the wire is M151d.
- `crates/hst-sim/src/shot/powered.rs`, `Table::powered(bounds, power, spin)`: each cell's hit (axis position) is
  flown in the vertical plane on `Ball::fly` (PS2 floats). The stock cell's flight gives the landing and the net
  crossing height. The new speed is round(stock × power), capped at the 12-bit field's 2047 (~268 mph). The
  elevation is re-solved on integer table units (−45°…+45°, false position plus halving) so the ball lands at the
  stock landing, using the lower, flatter solution. The ball must still pass the net at least as high as the stock
  cell, or 1.04 m (centre cord 0.914 + ball + 6 cm) when the stock passes higher. If it can't, the speed steps down
  toward stock in tenths (halving search); if no step works, the cell stays stock. Frames = stock frames + (new
  model frames − stock model frames), so curve profiles, serve wind and the CPUs' path frames follow the faster
  flight.
- Model check: with the record's spin, the stock cells of Big Chief's flat serve fly their table's own frames
  (≥ 95 % within one frame, most exactly; the landing sits on the target-axis value less the 15 cm pull). So the
  tables were authored on this same ball step, and "same landing" means the same landing in the game.
- `mod_match::power` (called from `apply` after `shots`) regenerates every table a powered class launches from.
  For serves: base and weak-toss tables and the special up1 slice table, not the underhand. For strokes and
  volleys: topspin, slice, flat and their timing variants (`Timing::regenerate`); lobs and drops stay stock. For
  smashes: both kinds. Serve and smash spin is the table's record spin; rally spin is `params::rally_spin` per
  cell. Tables are built on threads (one per table; sequential on wasm) and cached for the process by
  (character, table, power bits), so the next match with the same mod loads at once. Stock players never go
  through this.
- Bigger Chief v8 (opentennis-mods `effefce`, not pushed, not packaged): `"shots": {"serve": {"power": 1.4},
  "stroke": 9, "volley": 10}`. The standard's §6 documents the object form.

## Numbers
- Build: one table takes 1.0–1.5 s single-threaded (opt-level 3, ~16 flights a cell). Bigger Chief's 8 serve tables
  build in parallel during the match setup; `powered_serve_lands_in` finishes in 2 s including setup.
- Bot matches, 900 s, all serves (first and second) by the mod player:
  - pc07 body at scale 1, serve power 1: 22 of 35 in, top 150 mph. Power 1.4: 22 of 33 in, top 207 mph, 10
    serves of 189–207 mph landed in.
  - Bigger Chief (scale 1.5, stats S) at power 1: 33 of 52 in (14 net), top 150 mph. Power 1.4: 33 of 53 in (15
    net), top 209 mph, 17 serves of 180–209 mph in. His net faults come from his body/timing; power 1 has as many.
- An earlier rule (pass the net as high as the stock cell or 1.184 m) left almost every flat serve stock, because
  stock flat serves cross at 1.10–1.18 m in the plane. With it, powered serves reached only ~190 mph from mid
  cells.

## Tests
- `hst-sim` `shot::powered::tests::powered_serve_table`: model frames match the table's, build < 5 s, top speed
  ≥ 1.35×, same bits twice plus a pinned FNV hash of the 1.4× pc07 flat serve table (a generator change moves it;
  online peers must all run the same one), and power 2 stays under the field cap.
- `hst-play` `mod_match::tests::powered_serve_lands_in`: Bigger Chief v8 in a bot match. His serve tables are
  > 1.3× stock speed, the underhand and the other player's tables stay stock, a 200 mph+ serve lands in, and no
  table is built while the match plays (no per-hit cost).
- `hst-base` `mods::tests::bigger_disc_body`: manifest parsing of the object form.

## Not verified / not 1:1
- No check against the original. This is an addition on top (a mod-only feature; stock tables and replays are
  untouched), so there is nothing in the original to compare against.
- The net rule only looks at the vertical plane through hit and target. It doesn't know where along the net the
  ball crosses, so a powered shot crossing near the posts (1.07 m) can clip the cord where the stock one wouldn't.
- The build runs during the match setup (threaded), not ahead of it on the select screen. The first match with a
  powered mod loads ~1–1.5 s longer, and later ones in the same run hit the cache. No disk cache.
- Cross-platform bits: everything goes through PS2-float ops and integer search, but the pinned hash is only
  checked on x86_64 here, not under `tests/platforms.rs` (aarch64/wasm32). That fits M151d, which needs it.
- Strokes, volleys and smash at power ≠ 1 build and run, but only serves were played and measured in bot matches.
- Bigger Chief v8 is committed in opentennis-mods but not pushed or packaged. The installed game would reject the
  object form until this lands, so run `tools/rerig/hst_bigger_chief.py` (and push) after the merge.
