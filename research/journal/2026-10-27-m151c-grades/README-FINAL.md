# M151c: select card grades from the tables' speeds

## What landed
- `hst-sim` `Table::mean_speed(bounds, power, spin, step)`: the mean speed of `Table::powered`'s cells over every
  `step`th cell along each axis. Same per-cell generator, sampled (power 1 = the stock words). The cell placement is
  now one helper (`place`) that `powered` shares; the pinned hash in `powered_serve_table` still passes.
- `hst-base` `grades::power_gain(iso, mod)`: for each class with a §6 `power` ≠ 1 (serve, stroke, volley), takes the
  source character's topspin/slice/flat tables (the serve's base tables) and the spins the match gives them.
  Gain = (mean sampled speed at power − at 1) in mph × POW_PER_MPH. Cached per (character, class, power).
  ~110 ms for all three classes with the disc warm, one thread per table.
- `grades::scores`/`grades_of` take that `gain` and add it to the category's power term. Only its own category:
  Impact's serve term and Volley's ground-power term stay on the source row's POW, because the timing windows and
  net play don't change with the tables.
- Main menu: the startup roster, the Mods screen's `mod_card` (opens the ISO at `menu.add.iso`) and the store's card
  tool pass `power_gain`. Lobby downloads (no disc rows) are unchanged.

## Numbers (stock tables, all 3 kinds, mean mph vs POW)
- Serve: POW 3 → 76, 4 → 80–81, 6 → 89, 7 → 92, 8 → 98, 9 → 99–102, 11 → 105, 13 → 110, 14 → 106. Slope 0.305 POW/mph, r 0.967.
- Stroke: 70 → 83 mph over POW 4 → 16. Slope 0.937, r 0.993. Volley: 56 → 77 mph over POW 2 → 16. Slope 0.651, r 0.999.
- Pure speed can't stand in for POW. Flat serve top speeds put B's c08 (139 mph) level with C's 139, and
  mean speeds put c08 (99) below c09 (C, 102). So the score stays anchored to the source row's POW and adds the
  speed gained over that row's stock tables. Stock rows (gain 0) read back their disc grades exactly.
- c07 serve at power 1.4: gain +10.8 POW (mean ≈ +35 mph; some cells step down for the net) → S. His own serve
  alone reads A. Stroke ×1.15 from c04: +9.0 POW. Volley ×0.8: −7.5.

## Tests
- `grades::tests::faster_tables_grade_up`: the 1.4× c07 serve reads S (A without the gain), a faster stroke grades
  up and a slower volley down, power 1 gives zero, it's cached/deterministic, and it takes < 3 s.
- `rows_read_back_as_their_grades` (zero gain) still holds.

## Not verified / not 1:1
- Not compared with the original: the grades of a powered mod are our addition (the original has no such shots).
  Stock characters keep their disc grades, which the test checks.
- The gain is a sample (4×4×4 cells per table), not the whole generated table. It is the generator's own cells,
  but the mean can differ a little from the full table's.
- POW_PER_MPH is a constant fit to the 14 stock characters' mean table speeds (this journal), not read from the disc
  at run time.
- Smash power doesn't change any grade (no category for it).
- Not looked at on screen. The card shows `grades_of`'s output, which the test covers.
