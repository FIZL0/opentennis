# P114 — repo audit checks: unproven ports, a guessed argument, f64 rounding, two rewrites

From the 2026-10-21 repo audit (`../2026-10-21-repo-audit/README-FINAL.md` §6).

## Done

- **shade.rs inverse rewritten** in our own shape: the textbook adjugate from the six 2×2 minors of the top two
  rows and the six of the bottom two, over the determinant (still EE floats via `F`, so it stays deterministic).
  It is a few ulps and some zero signs off the game's (which lost them in its own op order); rendering only has to
  look right, so `tests/shade.rs` `light_matrices_match_the_game` now checks the 17 court 10 light matrices to a
  relative 1e-5 (the planes stay bit for bit; the receiver tiles/vertices tests still pass unchanged).
- **ai.rs's two many-argument ports** reshaped: `AiParams::reaction(t, last, Reacting { lob, special_serve,
  beside_human, near_net }, roll)` and `Stander { from, reach, side, depth, strong }.stand_side(minus, plus, width)`.
  Same behaviour (`tests/ai.rs` recorded decisions pass); the one `play.rs` call site changed in place.
- **npc.rs sound arguments confirmed** (no longer a guess). The type-19 sweep's play call passes the angle in the
  4th slot and `vol` in the 7th, a fixed 0x80 in the 5th. The sound layer clamps the 4th to 0..359 and its driver
  wraps it as a "RoundRange" (a bearing); the 7th and 5th both range 0..128. Which of those is the level: every
  other caller of the same play call (sounds played from a table of bearings) passes a bearing from that table in
  the 4th, always 0x80 in the 5th, and its varying level in the 7th (0x20, 0x40, 0x4c, 0x66, and once a 0..1
  fraction × 128). So (bearing, volume) = (angle, vol), as ported. Comment updated.
- **npc.rs deciding-set reset idle (type 44, msg 2)**: presentation (a court creature's idle animation and voice)
  on the court generator, which is its own MT reseeded every point from `rand()`; its draws can't reach the
  shared/AI generators or match timing. PLAN's in-match presentation rule asks only that it look right, so no
  forced-case proof is owed. Comment reworded.
- **f64 / to_radians sites**, where each feeds:
  - `player.rs` `Stats::new`: already a chop (`chop(spe as f64)`), proven by hst-oracle `float_steps.rs`
    `speed_is_spe_over_ten` (the TParam parser, past 2^24 too). Sim: exact.
  - `npc.rs` walker `slide` (`-r - 0.1` in double): the game's soft-float double, round to nearest, which Rust's
    f64 matches; `r` is the walker's one radius, and hst-oracle `walker.rs` runs the game's walker tick over 12k
    states bit for bit. Walkers don't move the ball or players.
  - `foot.rs` `throw` (`lunge × 0.3` in double): grass/clod bits of a dive, presentation; the number of `rand`
    draws doesn't depend on the value.
  - `rng.rs` flare angles and the menu flame speed: presentation (look-right plain floats, already documented as such);
    the draw counts don't depend on the values.
  - `camera.rs` `to_radians` (pitch, fov) and `cutaway.rs` `to_radians/to_degrees`: presentation (cameras and
    cut-aways only have to look right; `tests/camera.rs` holds the recorded view to 0.01). The camera never feeds the
    sim: the pad direction uses a fixed π flip for the +z side (`player::turned`), not the view.

## Split off

- **P115**: the dive slide's early end in `swing.rs` `Dive::step` is gameplay and still has no forced case. The
  game's routine is the per-player stroke update at 0x34ec70 (5.6 KB, ~20 callees: motion/animation helpers and the
  court mover at 0x34a960, which returns whether the step went through). The slide's stray test is the
  cos 25° compare in it. An oracle test needs a RAM dump with a player mid-dive slide, the mover hooked to clamp,
  and the motion calls stubbed. That's too big to fit next to the rest of this check, hence its own task.

## Not verified / not 1:1

- Dive slide early end (swing.rs): not proven here, split to P115 (see above).
- shade.rs light matrices: now a few ulps / zero signs off the game's by design (rendering rule), checked to 1e-5.
- Type-44 deciding-set idle and the type-19 sweep sound: never seen running against the original (presentation;
  the sweep's argument order is confirmed from the code's callers, not by ear).
- foot.rs / rng.rs / camera.rs / cutaway.rs float sites: not compared to the game bit for bit (presentation only).
