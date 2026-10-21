# P117 — smash and powered-shot aim gaps: spec phase

This session read the decompile and wrote `SPEC.md`; no Rust. The port phase implements it with the decompile
locked. Oracle entries for the four functions involved were added to `research/oracle.txt`.

## Findings (details in SPEC.md)

- **Smash deep mode:** a plain (✕/○) smash off its timing (|offset| > 1) gets a negative mode from how deep the
  swing started (0 inside 6.674 m, −10 at the baseline), scaled at the launch by Serv POW against SM LOW POW. The
  court pull takes its wider second margin pair (0.9 across, 0.75 along) when that blend is below 0. Strokes and
  volleys always pass 0, so their `low = false` is already right.
- **Powered shots' unscaled margins:** `special` = the special gate (`shot::special`) holds and the character's
  effect row for (class, kind) is non-zero: every character's clean stroke lob, and clean stroke slices by
  characters 3 and 8. Volleys and smashes never.
- **Smash table/blend:** the original always looks the smash up in its base table (the "blend" lookup reads the
  same table), so the port's base-table launch is exact. The blend only mixes the spin-group record toward the
  dw1 record, and on the disc those spin groups are identical, so it changes nothing observable.

## Not verified / not 1:1

- Nothing is verified against the game yet: this is the spec phase. The port phase checks it with the oracle
  and the smash/lob recordings (SPEC.md *Verification ideas*).
- The practice exception (one player, player index 1 hitting) is described but may not be reachable in the
  remaster's practice mode.
