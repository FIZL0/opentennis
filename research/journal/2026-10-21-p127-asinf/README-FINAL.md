# P127 — the game's asinf in ps2-float

## What was off

ps2-float's generic `libm::asinf` uses fdlibm's constants rounded to nearest. The SDK build the game links
(`libm::sce`) converted decimal literals toward zero: the same one-ulp-low polynomial constants and term order as
its `acosf`, and also `pio4_hi` one ulp low (0x3f490fda, not 0x3f490fdb). `pio4_hi` is only used on the
0.5 ≤ |x| ≤ 0.975 path, and that is where the 1–2 ulp misses near 0.7071 came from.

Worked out from fdlibm's public `e_asinf.c` and the toward-zero conversion of its decimal literals (the same rule
`sce::acosf` already follows). I didn't read the decompile; the oracle was the check.

## Done

- ps2-float **v0.3.1**: `libm::sce::asinf` (shares a `pq` helper with `sce::acosf`; past ±1 it returns +max, like
  the EE's 0/0). Test `asinf_near_the_diagonal_as_the_game` covers the threshold, ±1, past 1, tiny inputs, and stays
  within 2 ulps of the generic build. Docs and README list it.
- ps2-oracle **v0.1.4**: only bumps ps2-float, so this workspace keeps a single copy.
- Decomp kits (linux, windows): both crates bumped to ps2-float v0.3.1 / ps2-oracle v0.1.4; workspace tests pass;
  pushed.
- Here: `hst_sim::libm` re-exports `asinf`. New `hst-oracle/tests/libm.rs` `asinf`: 20k inputs, a quarter each from
  [-1, 1], within 32 ulps of ±1, within 300k ulps of 1/√2, and any finite bit pattern / [-8, 8]. All bit-exact. As a
  sensitivity check, with `pio4_hi` put back to 0x3f490fdb, 6484 of the 20k differ.
- `ball::gentle` is now the game's own test, `asinf(up) > π/4`, replacing the `GENTLE_UP` threshold constant.
  `rest_slope` (300k ulps around the threshold + 20k draws) still passes, and so does `line_calls`
  (`live_ball_comes_to_rest_as_the_game`).

No release note: nothing changes for the player (the rest check gives the same answers as before).

## Not verified / not 1:1

none. `asinf` is checked against the game's own function through the oracle, not with a PINE recording. It is a
pure function, so the oracle is the right proof.
