# P143: Bevy 0.19.1 → 0.20.1 (wgpu 29 → 30)

## What changed

- **Vendored crates.** bevy_audio, bevy_render, bevy_pbr, bevy_transform (P139), bevy_ecs (P142), wgpu and wgpu-hal
  are now the 0.20.1 / 30.0.1 releases, with every HST-marked patch merged back in by its marker.
  - P36: the multidraw GPU buffers upload only when a bin changes. In 0.20 this covers the bin-index and fan tables too.
  - P37: the bind-group cache now also covers 0.20's uniform-allocation bind groups.
  - P139/P142: carried over unchanged.
  - wgpu (B706 and the Metal precise-math switch): re-applied. wgpu-hal 30's `metal/device.rs` no longer imports
    `atomic`, so the precise-math check names `core::sync::atomic::Ordering`. The iOS build caught this.
  - winit stays at 0.30.13.
  - `third_party/README.md` table: updated. `tools/patches.py`: OK. `fuzz/Cargo.lock`: updated.
- **API migration**, per the 0.20 guide:
  - `iter_many` now returns a Result.
  - `iter_resources` now yields 3-tuples.
  - The `ExtractedWindow` component goes with `PrimaryWindow`.
  - `#[extract_app(RenderApp)]`.
  - Pipeline stages gained a `constants` field. P55's pipeline record stores and restores it.
  - `ui_widgets::Button` + `Pressed` on the start page.
  - `On<Add<T>>`, `get_aabb`, `BorderRadius::new`.
  - At most 16 system params.
- **Shaders on WESL.** 0.20 replaced naga_oil with WESL. Our five shaders were rewritten as `.wesl`:
  - gs, gs_prepass, hud_gamma, inspect, shadow/projected.
  - Imports now use `import crate::path::{…};` with 0.20's new module paths.
  - `#ifdef` became `@if`/`@elif`/`@else`, applied to statements, declarations, struct members and blocks.
  - `#{MATERIAL_BIND_GROUP}` became `constants::MATERIAL_BIND_GROUP`.
  - 0.20's compressed vertices: the shaders take `Vertex`, call `decompress_vertex`, and morph through the
    instance's descriptor index. The B277 guard is kept.
  - The three runtime patches of Bevy's own shaders (PBR's encoded output, the UI's two-channel letters, the UI
    sharpening) had silently stopped matching 0.20's `.wesl` files. They now target the new paths and use
    `Source::Wesl`.
- **Tonemapping.** 0.20's `Tonemapping::None` is a full passthrough: no tonemap pass in the shader, no dither, no clamp.
  `Tonemapping::Linear` is what 0.19's `None` did, so every camera that had `None` now has `Linear`. The look is
  unchanged (screenshots below).
- **P55 pipeline record.**
  - `VERSION` is now 4, since every shader was renamed and rewritten.
  - The hash skips imports that don't resolve. WESL lists each item import twice (module and module::item) and
    includes `@if`-guarded modules.
  - The 0.19 macOS seed (`pipelines-macos.json`) matched nothing any more. It is dropped, and `SEED` is `None` until
    it is re-recorded on a Mac (P144).
- **`Time<Virtual>`'s cap moved (0.20).** 0.19 clamped the real delta to `max_delta` and then scaled it by the
  relative speed. 0.20 clamps the scaled delta. As a result, the instant replay's skip (1000× speed, 1 s cap) ran only
  60 ticks a frame, and the full suite's `a_replay_skips_then_slows_in_frames` caught it. The replay pacing and
  offbeat's speed changes now scale `max_delta` by the speed, which is exactly 0.19's real-time cap. Rollback only
  sets its cap at speed 1, where nothing changed.
- **Weak set ordering (0.20).** 0.20's Render, RenderGraph and UI schedules order their sets weakly: systems with no
  data conflict can now run out of set order. I audited every system we add to those schedules:
  - `limit_present` (present pacing) and the `mark::<N>` timing probes touch no ECS data. They are now pinned with
    strong `.after`/`.before` orderings (Submit, Render, PrepareAssets…).
  - Everything else either has ECS conflicts that keep it ordered, or already uses explicit strong orderings.

## Verification

- **Tests:** clippy `--workspace --all-targets -D warnings` is clean. The targeted hst-base tests pass (pipelines,
  hud_gamma, gs, inspect, mips, stall, graphics). The full suite runs in done.sh.
- **Netplay:** `HST_NETPLAY=1 tools/check.sh -p hst-play netplay -- --include-ignored`: 48 passed. These include the
  ignored full-match netplay and rollback runs, `synctest_through_a_change_of_ends` and
  `a_silent_peer_rejoins_and_a_silent_host_ends_it`. The 2 that failed are manual tools that need a file
  (`replays_a_desync` wants `HST_DESYNC=…`, `hash_trace` wants `HST_HASH_TRACE=…`), not checks.
- **Other ignored runs:**
  - `every_match_length_plays_out` (bot matches at every length) passes.
  - `short_code_on_a_testnet` fails, on main (0.19) too: the CPU seats get [6, 12] and [7, 6] instead of the [0, 1] the
    test expects. Filed as B713; it is not a P143 regression.
  - hst-sim's replay tests don't depend on Bevy, so they run unchanged in done.sh's full suite.
- **Screenshots, 0.19 against 0.20** (`tools/shot.sh`, HST_WEATHER=0; in `context/p143/shots/`, `*-side.png`):
  - Menu: 356 of 2.36 M pixels differ by more than 24, almost all in the version label.
  - Court 1 and court 5 matches: 0.3% of pixels differ by more than 24, mean difference 0.3 / 0.25.
  - The only visible difference is a player portrait that has appeared by 16 s in 0.20 but not in 0.19 (load
    timing). In a 30 s shot of each, the HUD and portraits are identical.
- **Builds:**
  - Windows (MinGW) builds. It also runs under wine and draws the menu (`context/p143/shots/wine-menu.png`).
  - Android APK, iOS IPA, macOS and web (WebGL2 + WebGPU packs) all build. `tools/webcheck.sh` is clean.
- **Web run (headless chromium, SwiftShader, `tools/web_check.mjs`):** the import works and a match starts with no
  shader errors. Players and HUD draw, but not the court or sky. A 0.19 web build from main does exactly the same in
  this headless setup, so it is not a 0.20 regression. Visit 2 opening the site page is also old behaviour (M152b's
  capture shows it).
- **CPU** (`tools/bench.sh`, 4-bot doubles, `--fps 120 --scale 100 --shadows high --msaa 4x`, three interleaved
  rounds of 65 s each, first 10 s skipped, machine quiet with load < 1):

  | build | round 1 | round 2 | round 3 |
  |---|---|---|---|
  | 0.19 (main) | 21.1 | 21.1 | 20.7 s CPU / 60 s |
  | 0.20 | 17.3 | 16.8 | 17.0 s CPU / 60 s |

  0.20 is about 19% less CPU (2.9 → 2.35 ms/frame), and both hold 120 fps with 1% lows of 117–120. P142's ~33 s
  figure was taken under heavy load from other agents, so the A/B here is the fair comparison. Logs are in
  `context/p143/bench/`.

## Not verified / not 1:1

- **No samply profile.** samply isn't installed here, the same as in P140. CPU totals come from the HST_FPS log
  (`/proc/self/stat`), so I didn't check which call stacks the saving comes from.
- **B706's wgpu workaround on Adreno/Mali, and the iOS/Android builds in general:** compiled only. No phone or
  handheld is attached.
- **The start page's tap on 0.20's `Pressed`:** it compiles and the page draws, but I didn't tap or click it by hand,
  on a touch screen or otherwise.
- **The mesh metadata uniform-vs-storage buffer choice that 0.20 makes on WebGL2:** only checked through the web
  run above (players draw). That run can't show the court in either version, so court drawing on WebGL2 is unverified.
- **0.20's uniform-allocation bind groups have no dynamic offset.** P37's cache keys on the buffer alone; that is
  right by construction but not tested separately.
- **macOS seed dropped:** a fresh macOS install compiles every pipeline cold until P144 re-records it on a Mac.
- **`fuzz/Cargo.lock` has a `[[patch.unused]] ggrs` entry.** I didn't investigate it; the fuzz targets weren't run.
- **Native GL backend (`WGPU_BACKEND=gl`):** this machine finds no GL adapter, so it wasn't tried.
- **The first 0.20 bench round** had one 5 s window with 1% lows of 33 fps near the start, likely a pipeline compiling
  cold (P55 record version 4). Rounds 2 and 3 had none.
