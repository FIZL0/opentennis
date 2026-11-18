# P145 — Apple first-run pipeline seed

A fresh install on Apple hardware had no pipeline record (the 0.19 Mac seed was dropped in P143 because it named
`.wgsl` shaders that are gone), so the first menu and match compiled every pipeline under the loading screen.

## Done

- `crates/hst-base/src/pipelines-apple.json`: the user's iPhone 18 Pro `pipelines.json` (record v4, Bevy 0.20,
  device `Apple A20 Pro GPU | Metal`, 1 match on court 6, 94 pipelines over 19 WESL shaders), byte for byte.
- `SEED` includes it on iOS, macOS and in tests (was `None`); `collect` already re-tags a seed with the running
  device, so any Apple GPU takes it. `the_seed_is_this_format` runs again (format = v4, > 50 pipelines).
- No `.wesl` shader changed since the phone's build (git log), so the recorded source hashes should match this build;
  any that don't are left out by the P136 hash check (cost: a compile in play, never a crash).

## Not verified / not 1:1

- Not run on a device (none here): the done-when (fresh-install hst-log with no render stall > 100 ms at disc import
  → menu and match load) needs a new hst-log from the phone on a build with this seed. P146 already cut those stalls
  to ≤ 80 ms with no record at all (hst-log 8), so the seed mainly moves the remaining compiles into the menu.
- The record holds one match on one court (6): pipelines other courts or doubles ask for first (other shader
  defs) still compile on first use. Refresh the seed after matches on several courts.
- Not checked on a Mac: Mac GPUs may differ in shader defs/formats; mismatched entries only cost a menu compile.
- Shader hashes not checked against this build in a test (needs the full embedded shader set loaded).
- Rendering/performance only; no gameplay, sim RNG or match timing touched.
