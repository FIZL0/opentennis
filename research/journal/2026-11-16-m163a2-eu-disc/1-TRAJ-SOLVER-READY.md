# M163a2 — can the US trajectory tables be regenerated? No (blocked, needs a decision)

The user chose (b): the EU disc plays the US 60 Hz game. The first gate was to prove that a solver built on our
ported flight regenerates the US disc's TRAJ tables bit for bit. It doesn't, and the measurements below show why no
solver can without the authors' own offline tool.

Measurement: `crates/hst-sim/src/shot/powered/regen.rs`, `stock_serve_table_against_the_flight` (ignored; run with
`tools/check.sh -p hst-sim regen -- --ignored --nocapture`). Table: Big Chief's flat serve (`tr_pc07_serv2`), all
4,096 cells flown with the ported ball step (drag, Magnus, gravity on PS2 floats) and the serve record's spin.

## What the tables are

- The ported flight is the tables' physics, near enough: 3,766 of 4,096 cells (92%) fly exactly their stored frames.
  Every cell lands within −12.9…+7.5 cm of its target (mean about −6 cm on the lowest hit row, about 0 on the
  highest).
- A cell is one point on a one-parameter family. Each (hit, target) pair has a curve of (elevation, speed) pairs
  that land on the target, and the stored cell sits somewhere on it. Along a row the chosen point saw-tooths: the
  elevation climbs, drops back, then climbs again, while the frames go 50, 51, 51, 52, 55, 56, … No smooth rule
  (net clearance, apex, speed) explains it. Net clearance alone wanders 1.07–1.27 m.
- First guess: "pick an integer landing frame L, solve exactly, truncate". It fits a few cells to within rounding
  (z0 x2/x3: 173.4/350.9 → stored 173/350). Most cells sit 1–10 elevation units off every integer-L solution. Their
  landing times' fraction of a frame is spread evenly over the deciles (431, 401, 456, 414, 366, 405, 424, 383, 407,
  409), so landing frames weren't the search variable.
- Even with the stored elevation given, the speed that lands exactly on the target equals the stored speed in only
  76 of 586 sampled cells (13%) when truncated. Other assumptions don't help: ground at −0.05 m gives 90/586, ground
  at 0 gives 0/586, and a half-unit elevation rounding gives 58–77. The leftover spans about 6 speed units (6/1024
  m a frame), several times the rounding.

**Conclusion.** The authors' offline tool solved each cell by an iterative search and stopped once the landing was
within a tolerance (about ±5–10 cm). The stored bits are wherever that search happened to stop. That depends on the
tool's start points, step rule, iteration order and tolerance. None of these are on the disc: the game only loads
the tables, and GAME.BIN has no solver, only the debug string "target area over!!" from the lookup. A solver of our
own can land every cell on its target (as `powered.rs` already does for mods), but the result would differ from the
US tables in most cells by a few units. That is a different game in the small: shot speeds off by up to about 0.6%,
and frames off by 1 in some cells.

So (b) as asked ("EU players play exactly the US game, the online digest hashes the regenerated tables") can't be
done. The rest of (b) (shot params ÷1.2, camera words, court files, `SAITOU/TRAJ`) was gated on this and wasn't
started.

## Choices for the user

1. **(a) 50 Hz match mode**: play the EU disc as the EU game (M163a2's README: EU decompile, spec, per-release sim,
   switchable tick, recordings). Exact, but EU players can't play US/JP players online.
2. **(b′) approximate 60 Hz**: regenerate the tables with our own solver from the EU disc's targets. It plays the US
   game closely but not exactly: a new "EU 60 Hz" ruleset with its own digest. Cross-region online would then pair EU
   players only with each other (or both peers would use the regenerated set).
3. **(b″) a US peer's tables online only**: in a cross-region match, the US/JP peer sends its TRAJ tables to the EU
   peer for that match, which then plays exactly. This moves game data between players (never in git). Offline EU
   play would still need (a) or (b′). This is a policy call for the user.
4. **Leave Europe refused** (as now), and point EU players to the US disc.

Recommendation: (4) now, plus (b′) only if the user accepts "close, not exact" for EU-only play. (a) is the only
exact way, and it's the size of the JP work or bigger.

## Not verified / not 1:1

- Only one table (pc07 `serv2`) was measured cell by cell. The saw-tooth and the tolerance-sized scatter are visible
  by eye in pc00 `strk0` and `serv0` too, but they weren't run through the flight. Stroke tables also need the
  per-kind spin shaping (`params::rally_spin`), which this test leaves out.
- Search schemes not tried: dyadic bisection grids (they can't be told apart from truncation at this resolution),
  or other drag/gravity variants (with 92% exact frames, the physics isn't the gap).
- Nothing ported. Europe stays refused. No PCSX2 run (none needed: this is all offline data).
