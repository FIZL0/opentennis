# P117 — port phase 2: the special (unscaled) margins

Ported SPEC.md C from the spec round 2 answer, decompile locked.

## What landed

- `hst_sim::shot::has_effect`: the character's effect row for (class, kind) bends or curves (stroke slice with
  a rally-slice bend, stroke lob with a lob curve).
- The strike (`play.rs`, `special_margins`) pulls a rally shot with `special = has_effect(src's effects) &&
  shot::special(striker's grade/offset)`, replacing the old `special = false`. The gate is the striker's own lock,
  which the spec's live probe showed equals the game's latched copy at every hit.
- Oracle (`hst-oracle/tests/court_pull.rs`, `court_pull`): now writes random latched copies (grade 0..3,
  offset −4..4) for all four players and a random last hitter, and the port's `special` from the hitter's copy:
  20k pulls, 0 wrong, 18 moved by unscaled margins (40 in `court_pull_forced`). Mutation check: reading the next
  player's copy instead of the hitter's gives 17 wrong.
- Recording tests use the real flag too: `timing.rs` `timed_launches_like_the_game` (14 special launches of 260,
  all exact) and `shot_tables.rs` `rally_aims_pulled_inside_like_the_game` (13 special launches, all exact).

## Not verified / not 1:1

- No recording has a special launch whose aim reaches a line, so the unscaled margins' *effect* rests on the
  oracle (20k pulls), not on a recorded shot. No new PCSX2 recording made (a human clean cross-court lob aimed
  at a corner would show it).
- The latched copy is modelled as the striker's live grade/offset (spec C.1): a swing making contact with a
  999/9999 offset would read a stale copy in the original; none seen in the probe.
- The practice exception (one player, player index 1 hitting) is left out, as in `shot::effect`: the remaster's
  practice exception is not modelled (not checked whether its practice can have player index 1 hitting).
- The blend's magnitude, the body-shot addition and D as in 2-PORT-FINAL.md.
