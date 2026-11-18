# Vendored crates

Each folder is the crates.io release named below with our patches applied, wired in by `[patch.crates-io]` in the root
`Cargo.toml` (and the same list again in `fuzz/Cargo.toml`, its own workspace; `tools/patches.py` checks the two
match and that every file changed from upstream is listed here and carries a marker). Every change is marked in
the source with a comment naming its task: `HST (…)`, `HST patch`, `hst B…`/`hst P…` or `opentennis …`
(`grep -rnE '\b(HST|hst|opentennis)\b' third_party`).

To move one onto a new upstream release: unpack the release (`cargo download`, or the copy in
`~/.cargo/registry/src/*/<crate>-<version>`), carry over the files below by their markers, update the version here
and run `tools/patches.py`. A patch can go once upstream fixes the same thing.

Upstream status: none of these has been sent upstream yet (2026-10-21).

| Crate | Upstream | Files changed | Task: what |
|---|---|---|---|
| `bevy_audio` | 0.20.1 | `src/audio_output.rs`, `src/lib.rs` | B677: the output reopens when its device goes away (PipeWire restart, suspend, Bluetooth headset off): the sink's error callback logs once and raises a flag (rodio's default logs every error, thousands a second once ALSA's poll fails), a silent heartbeat source catches a device that just stops pulling, and `reopen_audio_output` opens the default sink again on a thread about once a second, queues the playing sounds on the new mixer and sends `AudioOutputReopened` |
| `bevy_render` | 0.20.1 | `src/render_phase/mod.rs`, `src/batching/no_gpu_preprocessing.rs`, `src/view/window/mod.rs` | B385: on CPU batching (WebGL2) an unbatchable mesh with no batch data this frame keeps its slot and is skipped, instead of shifting the dynamic offsets past their end. Upstream has the same bug (no issue found). `src/view/window/mod.rs`: Mailbox (the game's VSync on Linux) falls back to Fifo, never Immediate, so it never tears. |
| | | `src/batching/gpu_preprocessing.rs` | B395: the same slot-keeping on the GPU-preprocessing path |
| | | `src/render_phase/mod.rs` (`dirty`), `src/batching/gpu_preprocessing.rs` (`write_binned_instance_buffers`) | P36: a retained bin's GPU buffers (its instances, bin metadata, fan and bin-index tables) are uploaded only when an entity came into or left one of its bins |
| | | `src/render_resource/pipeline_cache.rs` (`compile_budget`) | P47: where pipelines compile on the render thread (browser, macOS) a queue pass stops after a time budget, set by `crates/hst-play/src/mode.rs` |
| | | `src/render_resource/pipeline_cache.rs` (`dedup`, `prewarm_render_pipeline`) | P55: queueing a render pipeline equal to a cached one returns its id; pre-warmed ones (`crates/hst-base/src/pipelines.rs`) aren't `requested` until the game asks |
| | | `src/render_resource/pipeline_cache.rs` (`block_on_render_pipeline`, `take_new_pipelines`) | P146: a pipeline the render thread must wait for (the upscaling blit for a new view) compiles alone, in place, instead of after a queue pass that spawned every other queued compile ahead of it on the async pool (they take the shader cache one at a time, so the frame sat through all of them: 440-560 ms on iOS at menu and match loads) |
| | | `src/render_resource/pipeline_cache.rs` (`NO_CUBE_ARRAY_TEXTURES_SUPPORT`) | M154: that shader def on ARM Linux too (see bevy_pbr's `light.rs`) |
| | | `src/renderer/mod.rs` (`PrimaryDisplay`), `src/settings.rs` (`create_render`) | M154: on ARM Linux (GL ES over EGL) the wgpu instance gets the primary window's display, without which its surface had no present modes; so the renderer is created off the main thread once winit has made the window, as in the browser |
| | | `src/view/mod.rs` | B438: the main texture's sRGB `view_formats` only where the adapter has `VIEW_FORMATS` (WebGL2 hasn't, and the texture failed to create) |
| | | `src/lib.rs` (`gpu_preprocessing_blocked`, `choose_skin_buffers`, `skins_use_uniform_buffers`), `src/renderer/mod.rs`, `src/batching/gpu_preprocessing.rs`, `src/mesh/morph.rs` | B706: where Bevy's driver list sends a GPU to CPU preprocessing (Adreno ≤ 730, Mali drivers < 48; GitHub #4: an Adreno 730 drew every skinned mesh exploded or not at all), skins and morph weights use WebGL2's uniform-buffer layout; `HST_CPU_PREPROCESS=1` and the game's `GPU_PREPROCESS_SWITCH` / `UNIFORM_SKINS_SWITCH` (`debug.txt`) force either way |
| `bevy_transform` | 0.20.1 | `src/systems.rs` | P139: transform propagation always uses the serial walk. The parallel walk's workers spin on its work queue while one thread walks a deep chain (a character's skeleton): ~40% of the game's CPU on every compute thread, every frame (desktop profile 2026-11-16: 37 s of 86 s CPU over 80 s of a match; serial: 0.8 s), which drains phone batteries |
| `bevy_ecs` | 0.20.1 | `src/query/par_iter.rs` | P142: `par_iter` over fewer than 4096 items runs on the calling thread, as the web build always does. A parallel pass woke every compute thread for microseconds of work; at our entity counts the wake-ups and parking (futex calls, queue spinning) cost far more than the work, every frame, in a dozen of Bevy's systems (visibility, material specialization, mesh extraction) |
| | | `src/world/identifier.rs` | P141: `WorldId::new` allows its deprecated `fetch_update`; B711: it calls `try_update`, the pinned compiler's name for it (0.20.1 does so upstream under `std`; the allow covers the `no_std` branch) (Rust 1.99 warns, and tools/webcheck.sh fails on any warning) |
| `bevy_pbr` | 0.20.1 | `src/render/gpu_preprocess.rs` | P37: the mesh-preprocess bind groups (and 0.20's uniform-allocation ones) kept while their buffers stay the same (bin-unpacking metadata at a dynamic offset), not rebuilt per view and phase each frame |
| | | `src/render/skin.rs` (`skins_use_uniform_buffers`) | B706: asks bevy_render's `skins_use_uniform_buffers` (above) |
| | | `src/render/light.rs`, `src/render/mesh_view_bindings.rs` | M154: ARM Linux's point-light shadow map is a Cube view, not a CubeArray (GL ES can't bind one; the iOS simulator's path) |
| | | `src/render/light.rs` (`directional_light_shadow_map_texture`) | M155b: on ARM Linux the directional/spot shadow map has at least two layers, so GL ES makes it a texture array its D2Array view can bind |
| | | `src/render/light.rs`, `src/lib.rs` (`ReusesShadowMaps`) | Split screen: a camera with this marker draws no directional shadow pass and samples the one the camera before it drew (its owner gives both the same cascades, `twin_view::shared_cascades`) |
| | | `src/render/light.rs` (`extract_shadow_lod_origin`) | M157: no shadow LOD origin looked up while no point or spot light exists (its query lenses walked every archetype each frame) |
| | | `src/ssao/mod.rs` | M154: SSAO isn't set up on the GL backend (naga's GLSL for its depth gather doesn't compile, and the failed pipeline quit the app); the game uses no SSAO |
| | | `src/cluster/mod.rs` | B468: CPU clustering reuses each view's cluster buffers instead of new ones every frame (WebGPU frees them only at GC) |
| `wgpu` | 30.0.1 | `src/backend/webgpu.rs` | B328: WebGPU device features/limits read once, not from the browser on every draw. Upstream 30.0.1 still reads them on every draw |
| `wgpu-hal` | 30.0.1 | `src/gles/egl.rs` | M155b: a surface on an OhosNdk handle gets a back buffer (in a single buffer the Mali fbdev display never showed a frame) and (the Mali fbdev window on ARM Linux, `crates/hst-base/src/fbdev.rs`) is presentable even though the blob EGL marks no config `NATIVE_RENDERABLE`, as wgpu already does on OpenHarmony |
| | | `src/metal/mod.rs` (`PRECISE_MATH`), `src/metal/device.rs` | B693: Metal shaders compile without fast math while the game sets `PRECISE_MATH` (`debug.txt`'s `metal_precise_math = on`, `crates/hst-base/src/mod_debug.rs`), to test whether fast math is behind the see-through mod heads on iOS |
| `winit` | 0.30.13 | `src/platform_impl/ios/window.rs`, `src/platform_impl/ios/event_loop.rs` | M91: iOS 27's UIScene lifecycle; windows join the app's scene once it connects |
| | | `src/platform/ios.rs` (`ValidOrientations::LandscapeRight`), `src/platform_impl/ios/view_controller.rs` | B694: a landscape-right-only orientation, which the game holds under an iPhone's rotation lock |
| | | `src/platform/android.rs` (`gamepad`), `src/platform_impl/android/mod.rs` | M101: Android pad key/motion events go to a queue for `crates/hst-base/src/android_pad.rs`, not into Touch/KeyboardInput |
| | | `src/platform/android.rs` (`lifecycle`), `src/platform_impl/android/mod.rs` | M108: an activity lifecycle hook (start/resume/pause/stop/destroy) for `crates/hst-base/src/android.rs` |
| `ggrs` | 0.13.0 | `src/clock.rs` (new), `src/lib.rs`, `src/network/protocol.rs`, `Cargo.toml` | B494: ggrs' clock can be driven by the calling thread, so netplay's round-based tests get the same pings, resends and time sync however long a round takes. `Cargo.toml` drops the examples/tests/dev-dependencies the vendored copy doesn't ship |
| `gilrs-core` | 0.6.8 | `src/platform/ios/` (new), `src/platform/mod.rs`, `Cargo.toml` | M91: an iOS backend over Apple's GameController framework (`objc2`) |
| | | `src/platform/linux/gamepad.rs` (`find_buttons`) | M155b: `GILRS_SDL_ASCENDING=1` numbers a pad's buttons in key-code order, as muOS's SDL and its gamecontrollerdb do (its virtual pad's volume keys come first there), so PortMaster's mapping lines up |
| | | `src/platform/linux/gamepad.rs` | Doubles play (2026-10-06, no task id): a pad whose `/dev/input` node is read-only still works for input, without force feedback |
| | | `src/lib.rs`, `src/platform/default/ff.rs` | B618: no unused-import / unused-variable warnings in the Android and iOS builds |

Not code: the vendored `Cargo.toml.orig`/`Cargo.lock` files are upstream's and never read; `LICENSE-*` files are
kept (gilrs-core's added from upstream's repo, which the crate package lacks).
