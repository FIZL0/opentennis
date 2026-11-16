# P144 — re-record the macOS pipeline seed on Bevy 0.20

**Blocked: needs a Mac.** This unattended run is on Linux (Vulkan). No Mac can be reached from here: `~/.ssh/config`
lists no hosts, and nothing in `plan/REFERENCE.md` or the tools describes remote Mac access.

A seed recorded on Linux would be wrong. A record holds its pipelines' descriptors, including the surface/target
formats and the backend's shader set, and the M3 seed exists because Metal starts cold. A Vulkan record would warm
the wrong pipelines or none at all. So this needs a real Mac recording; nothing was changed in code.

## What the human does (about 5 min)
1. On a Mac, build main (`cargo run --release -p hst`), delete any old `pipelines.json` in the app's data dir, then
   walk the menus and play one match (singles and doubles if possible, so `KEEP` gets both).
2. Copy the recorded `pipelines.json` to `crates/hst-base/src/pipelines-macos.json`.
3. In `crates/hst-base/src/pipelines.rs`, set `const SEED: Option<&str> = Some(include_str!("pipelines-macos.json"));`.
4. `tools/check.sh -p hst-base pipelines`: the seed test checks that the version is 4 and that it parses.

## Not verified / not 1:1
- Nothing was recorded or changed: a macOS/Metal machine is required and none is available to the runner.
