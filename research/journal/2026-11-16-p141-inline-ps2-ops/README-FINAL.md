# P141 — Inline the PS2 float ops

## What changed
- ps2-float **v0.3.2**: `#[inline]` on every function in `fpu.rs` and `vu.rs` (70 + 64; the few that had it
  kept theirs). No code change otherwise. ps2-float's own tests pass, clippy clean.
- ps2-oracle **v0.1.5**: only the ps2-float bump.
- Here: `hst-sim` → ps2-float v0.3.2, `hst-oracle` → ps2-oracle v0.1.5 (the lockfile keeps one ps2-float).
- Decomp kits (linux, windows): `crates/game-sim` and `crates/kit-oracle` bumped to the same two tags, their
  workspace tests pass, pushed to their main.

## Measured (desktop, release profile = no LTO, machine shared with other agents; alternating runs)
- Costume sway: a scratch bench calling `hst_sim::noise::deform` (what `hst_base::noise::draw` runs per distinct
  entry) 20 M times: 394–445 ns → 320–328 ns per call with ps2-float 0.3.1 → 0.3.2, ~12–17 % less, same output
  hash. `nm` on the bench: 0.3.1 has `vu::add`, `vu::mul`, `vu::sub`, `vu::madd`, `Lane` ops as called symbols;
  0.3.2 keeps only `vu::add` out of line (the compiler's choice: TwoSum + chop is big), the rest inlined.
- Sim: `state_hash fixtures/camera_s05.bin` (a recorded bot match through the match camera + the libm sweeps):
  0.115–0.123 s → 0.103–0.107 s, ~12 % less; the 6021 output lines are byte-identical.
- What remains in `deform` is the exact-rounding arithmetic itself (f64 TwoSum, chop), not call overhead.

## Tests
- `tools/check.sh -p hst-oracle` (36) and `-p hst-sim` (131) pass on the new tags; full suite in done.sh.

## Not verified / not 1:1
- No samply profile of the full game with mod characters (the P138 setup: a 4-mod-character match in a window):
  measured instead with the deform micro-bench and `state_hash`, which time exactly the code that was inlined and
  show the call symbols gone. The in-game ms/s figure for `noise::draw` after the change isn't re-measured.
- Results can't differ (inlining doesn't change IEEE rounding in Rust: no fast-math, no FMA contraction); checked by
  identical bench hashes, identical state hashes and the passing replay/oracle tests.

## Also
- done.sh's web check failed on a warning that wasn't from this task: P142 vendored bevy_ecs into `third_party/`,
  and its `WorldId::new` calls `AtomicUsize::fetch_update`, deprecated on our toolchain; `tools/webcheck.sh` counts
  `third_party/` warnings as ours. Fixed with one `#[allow(deprecated)]` on that function (marked as ours).
