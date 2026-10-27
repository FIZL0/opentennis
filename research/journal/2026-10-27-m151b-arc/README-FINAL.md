# M151b: §6 `shots` `arc` and `preset`

Branch note: task/M151b was cut before M151a landed; fast-forwarded to main (ad243184) to build on its generator.

## What landed
- Manifest (`mods::read`): a `shots` entry object takes `preset`, `power`, `arc` (and `from`). `arc` 0.25–2.5
  (1 = as `power` flies it), stored in `Build::arc`. `preset` (`mods::PRESETS`) sets power/arc: cannon 1.4/0.75,
  laser 1.2/0.5, heavy 1.1/1.5, moonball 0.8/2, soft 0.9/1 (flat = 0.75, very flat = 0.5, high = 1.5, very high = 2,
  stock = 1). An explicit `power`/`arc` beside a preset wins. Unknown preset, non-string preset, arc out of range:
  rejected. Any arc ≠ 1 makes `Build` non-default, so the online lobby refuses it (M151d carries it).
- Why spin: once a cell's speed (stock × power) and landing are fixed, the elevation is fixed too. The only free
  input left is the launch spin (Magnus), and more topspin dips the ball, so the same landing at the same speed
  takes a higher flight. `arc` is therefore a spin change, one amount per table:
  `Table::arc_spin(bounds, power, arc, spin(hit, target, change))` generates 27 sample cells (3×3×3 over the axes,
  through the real cell builder, net rule and speed step-down included) and halves a spin change in ±360 record
  degrees until their mean height over the centre cord is `arc` × its height at change 0. An arc out of reach takes
  the range end. `Table::shaped(bounds, power, stock_spin, spin)` builds the table. Each cell keeps the landing its
  stock spin gives and is solved at the moved spin. `powered` is now `shaped` with the same spin twice, and the
  M151a pinned hash is unchanged. A cell with a moved spin that can't clear the net at any step falls back to the
  stock speed re-solved at the new spin (net or not), not the stock word, which would land elsewhere.
- Match (`mod_match::power`): cache key gains the arc bits and the cache stores (table, change). After the build,
  the launch spin moves by the same change. Serves: each table's tuple spin (and up1's). Smashes: `smash_spins`.
  Strokes and volleys: the record's SPIN field in `rally_records` and in each timing variant's record
  (`Timing::regenerate` now hands `&mut` records), so `params::rally_spin` at the hit gives exactly the spin the
  table was built for (flat strokes' low/near-net lift included).
- Standard §6 (opentennis-mods b5c6f1c, committed on its main, not pushed) documents arc, preset and the table.

## Numbers (pc07 flat serve, power 1.4; mean height over the cord, all cells)
- own 0.213 m. arc 0.6 → 0.131 m (0.61×, change −95°: the record's +52° becomes backspin), 98 % of cells within
  5 cm of the stock landing. arc 1.5 → 0.320 m (1.50×, +125°), arc 2 → 0.425 m (1.99×, +201°), 100 % kept.
- A table with arc takes ~1.3–3.3 s (calibration + build) single-threaded. Setup of the preset test match (heavy
  serves + moonball strokes, built on threads) 5.7 s.
- Bot match (`preset_shots_land_in`): serves [Out, In, In], 23 of 23 rally shots of the moonball player In.

## Tests
- `hst-sim` `shot::powered::tests::arc_serve_table` (arc 0.6 and 1.5 on the table above, landings kept, arc 1 = no
  change); `powered_serve_table` hash unchanged.
- `hst-play` `mod_match::tests::preset_shots_land_in`; `powered_serve_lands_in` still passes.
- `hst-base` `mods::tests::bigger_disc_body`: preset/arc parsing, overrides, bad values.

## Not verified / not 1:1
- Nothing to compare with in the original: this is a mod-only addition. Stock characters never reach this code,
  and arc = 1 builds the same bits as M151a.
- Flattening is limited by the net rule (M151a's: pass as high as the stock cell or 1.04 m). Very flat on an
  already-flat powered serve reaches about the floor, and cells below it step their speed down, so "laser" serves
  can lose some of their power. The serve test checks only that the table gets flatter for arc < 1, not the exact ratio.
- One spin change per table, calibrated on the mean of 27 cells. Individual cells' heights spread around the target
  ratio.
- More spin also changes the bounce (topspin kicks higher and faster, backspin skids). That's intended ("heavy"),
  but no one has tuned it by eye in a played match.
- Smash and volley arcs build and launch through the same path, but no test exercises them. Only serves and strokes
  were played.
- The standard edit is committed in opentennis-mods but not pushed (the task forbids pushing).
