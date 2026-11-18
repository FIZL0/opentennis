# P145: Apple first-run pipeline seed

`crates/hst-base/src/pipelines-apple.json` is the user's iPhone 18 Pro record (beta build 5067, fresh install, menu +
one match on court 6 (refreshed after a second on court 10: 96), Phone preset: 67 % scale, shadows low, MSAA 2x): 94 pipelines, 19 shaders, format 4, only
`embedded://` shader paths (no device paths). It's the `SEED` on every Apple target (iOS and macOS, any Apple GPU,
all Metal; `collect` re-tags it with the device's own name) and in tests.

Why: with no record, a fresh install compiled every menu/match pipeline at first use (hst-log 6/7). P146 removed the
render-thread wait on those compiles; the seed means they're compiled during the menu instead of at load.

Limits: one court and the Phone preset only. Pipelines another court, weather or a Mac's settings (different MSAA,
shadows) ask for still compile on first use (async on iOS, in the frame on a Mac) and land in that device's own
record. Entries a device never asks for cost one compile in the menu each. Refresh the seed from a `pipelines.json`
after matches on several courts. A shader edited since build 5067 drops its seeded pipelines (source hashes, P136),
so a stale seed never fails validation.

Not yet verified on the device with the seed in: needs a fresh install of a beta built from this commit (expect no
render frame over ~50 ms at boot, menu or match load, and `pipelines: 94 recorded for this GPU` at start).

## Verified (hst-log 9, beta build 5070, fresh install)

`pipelines: 94 recorded for this GPU`, the record fed with 0 skipped; match load cover down after 6 frames (99 without
the seed, hst-log 8). In play: no frame over 19.7 ms (hst-log 8's 28.7 / 32.4 ms early frames gone). The seed was
then refreshed from that run's pipelines.json (courts 6 + 10, 96 pipelines).
## Second pass (parallel task branch, same seed byte for byte)

- Background: the 0.19 Mac seed was dropped in P143 because it named `.wgsl` shaders that are gone, so Apple hardware
  had no seed at all before this.
- No `.wesl` shader changed since the phone's build (git log), so the recorded source hashes should match this build;
  any that don't are left out by the P136 hash check (cost: a compile in play, never a crash).
- Not checked on a Mac: Mac GPUs may differ in shader defs/formats; mismatched entries only cost a menu compile.
- Shader hashes are not checked against this build in a test (needs the full embedded shader set loaded).
- Rendering/performance only; no gameplay, sim RNG or match timing touched.
