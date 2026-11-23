# 50 Hz match speed off the menu

User 2026-11-23: drop the choice, keep the timing code for the recorded European bot match.

- `FIFTY_HZ_SIM` is off: the Match Speed row is hidden, a saved choice plays at 60, a 50 Hz lobby turns us away.
- `set_current` also clamps to 60, so `--match-hz 50` (by hand or from a match saved at 50) can't run the clock at 50
  on the 60 Hz rules. `test_sim_hz` still plays 50 Hz in the tests.
- Before deciding: with the US disc lent, the European disc's 60 Hz bot match equals the US one tick for tick
  (`europe_lent`); the European disc alone is close, not exact (rebuilt shot tables, M170).

## Not verified / not 1:1

- The European disc alone at 60 Hz stays inexact (M170); the exact 50 Hz match can no longer be chosen.
