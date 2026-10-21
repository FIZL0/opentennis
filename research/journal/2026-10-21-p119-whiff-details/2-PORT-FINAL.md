# P119 — whiff details: port phase

Ported from `SPEC.md` with the decompile locked.

- `Reach` gains `smash_middle`: TParam column 64's middle cell ÷ 100, parsed like top and bottom (character 0's base
  reach from the cell, every player's `reach_of` from `ReachStats::smash[1]`, so a mod's comes from its own row).
- `swing::whiff_horizon`: the hitter's own grade count, at least 16. The whiff in play.rs builds its path with it
  (was character 0's 28) and compares against the hitter's own `smash_middle` (was `(top + bottom) / 2`).
- Test `tests/swing.rs::whiff_uses_own_horizon_and_middle_cell`: a 40-frame reach smashes at a ball that rises
  above the middle only after frame 28, a 28-frame one doesn't; a height between the middle cell and the midpoint
  flips to a smash; the comparison stays strict; a short table is floored at 16.
- `recorded_whiffs` and hst-play's whiff tests pass unchanged; the full suite (replays) runs in done.sh.

## Not verified / not 1:1

- No new PINE recording: a retail difference shows only for characters 7, 9 and 10 on a path sample at exactly the
  one-ulp height, and the horizon only for mods (no retail grade table differs in the way that matters here
  without a mod), so neither can be forced on the real game cheaply. Both rest on the spec and the unit test.
- The test literals in `tests/swing.rs` read the middle cell from player RAM between the top and bottom words; none of
  those tests compares it, so that offset is not checked.
