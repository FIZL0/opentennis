# P119 — whiff details: spec phase

This session read the decompile and wrote `SPEC.md`; no Rust. The port phase implements it with the decompile
locked.

## Findings (details in SPEC.md)

- **Path horizon:** the smash-whiff test walks the press's path, sized by the hitter's own grade count (raised to
  at least 16 frames), not character 0's 28.
- **Middle height:** the hitter's own TParam column 64 middle cell, read from the table. The port's
  `(top + bottom) / 2` is one ulp above it for retail characters 7, 9 and 10 (because of how the port parses and
  adds), so the gap shows on retail data too, not only for mods (in practice only on a sample at exactly that
  height).

## Not verified / not 1:1

- Nothing is checked against the game yet: this is the spec phase. The port phase checks it with a unit test
  (a reach whose count isn't 28 and whose middle isn't the midpoint) and the existing `recorded_whiffs` and
  replay tests.
- The one-ulp gap for characters 7, 9 and 10 comes from working out f32 values (chopped ÷ 100, native add/halve)
  from the TParam cells, not from a run of the port.
