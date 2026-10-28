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
| `bevy_audio` | 0.19.1 | `src/audio_output.rs`, `src/lib.rs` | B677: the output reopens when its device goes away (PipeWire restart, suspend, Bluetooth headset off): the sink's error callback logs once and raises a flag (rodio's default logs every error, thousands a second once ALSA's poll fails), a silent heartbeat source catches a device that just stops pulling, and `reopen_audio_output` opens the default sink again on a thread about once a second, queues the playing sounds on the new mixer and sends `AudioOutputReopened` |
| `bevy_render` | 0.19.1 | `src/render_phase/mod.rs`, `src/batching/no_gpu_preprocessing.rs`, `src/view/window/mod.rs` | B385: on CPU batching (WebGL2) an unbatchable mesh with no batch data this frame keeps its slot and is skipped, instead of shifting the dynamic offsets past their end. Upstream has the same bug (no issue found). `src/view/window/mod.rs`: Mailbox (the game's VSync on Linux) falls back to Fifo, never Immediate, so it never tears. |
| | | `src/batching/gpu_preprocessing.rs` | B395: the same slot-keeping on the GPU-preprocessing path |
| | | `src/render_phase/mod.rs` (`dirty`), `src/batching/gpu_preprocessing.rs` (`write_binned_instance_buffers`) | P36: a retained bin's GPU buffers are uploaded only when they changed |
| | | `src/render_resource/pipeline_cache.rs` (`compile_budget`) | P47: where pipelines compile on the render thread (browser, macOS) a queue pass stops after a time budget, set by `crates/hst-play/src/mode.rs` |
| | | `src/render_resource/pipeline_cache.rs` (`dedup`, `prewarm_render_pipeline`) | P55: queueing a render pipeline equal to a cached one returns its id; pre-warmed ones (`crates/hst-base/src/pipelines.rs`) aren't `requested` until the game asks |
| | | `src/render_resource/pipeline_cache.rs` (`NO_CUBE_ARRAY_TEXTURES_SUPPORT`) | M154: that shader def on ARM Linux too (see bevy_pbr's `light.rs`) |
| | | `src/renderer/mod.rs` (`PrimaryDisplay`), `src/settings.rs` (`create_render`) | M154: on ARM Linux (GL ES over EGL) the wgpu instance gets the primary window's display, without which its surface had no present modes; so the renderer is created off the main thread once winit has made the window, as in the browser |
| | | `src/view/mod.rs` | B438: the main texture's sRGB `view_formats` only where the adapter has `VIEW_FORMATS` (WebGL2 hasn't, and the texture failed to create) |
| `bevy_pbr` | 0.19.1 | `src/render/gpu_preprocess.rs` | P37: the mesh-preprocess bind groups kept while their buffers stay the same (bin-unpacking metadata at a dynamic offset), not rebuilt per view and phase each frame |
| | | `src/render/light.rs`, `src/render/mesh_view_bindings.rs` | M154: ARM Linux's point-light shadow map is a Cube view, not a CubeArray (GL ES can't bind one; the iOS simulator's path) |
| | | `src/render/light.rs` (`directional_light_shadow_map_texture`) | M155b: on ARM Linux the directional/spot shadow map has at least two layers, so GL ES makes it a texture array its D2Array view can bind |
| | | `src/ssao/mod.rs` | M154: SSAO isn't set up on the GL backend (naga's GLSL for its depth gather doesn't compile, and the failed pipeline quit the app); the game uses no SSAO |
| | | `src/cluster/mod.rs` | B468: CPU clustering reuses each view's cluster buffers instead of new ones every frame (WebGPU frees them only at GC) |
| `wgpu` | 29.0.4 | `src/backend/webgpu.rs` | B328: WebGPU device features/limits read once, not from the browser on every draw. 30.0.1 still reads them each time |
| `wgpu-hal` | 29.0.4 | `src/gles/egl.rs` | M155b: a surface on an OhosNdk handle gets a back buffer (in a single buffer the Mali fbdev display never showed a frame) and (the Mali fbdev window on ARM Linux, `crates/hst-base/src/fbdev.rs`) is presentable even though the blob EGL marks no config `NATIVE_RENDERABLE`, as wgpu already does on OpenHarmony |
| `winit` | 0.30.13 | `src/platform_impl/ios/window.rs`, `src/platform_impl/ios/event_loop.rs` | M91: iOS 27's UIScene lifecycle; windows join the app's scene once it connects |
| | | `src/platform/android.rs` (`gamepad`), `src/platform_impl/android/mod.rs` | M101: Android pad key/motion events go to a queue for `crates/hst-base/src/android_pad.rs`, not into Touch/KeyboardInput |
| | | `src/platform/android.rs` (`lifecycle`), `src/platform_impl/android/mod.rs` | M108: an activity lifecycle hook (start/resume/pause/stop/destroy) for `crates/hst-base/src/android.rs` |
| `ggrs` | 0.13.0 | `src/clock.rs` (new), `src/lib.rs`, `src/network/protocol.rs`, `Cargo.toml` | B494: ggrs' clock can be driven by the calling thread, so netplay's round-based tests get the same pings, resends and time sync however long a round takes. `Cargo.toml` drops the examples/tests/dev-dependencies the vendored copy doesn't ship |
| `gilrs-core` | 0.6.8 | `src/platform/ios/` (new), `src/platform/mod.rs`, `Cargo.toml` | M91: an iOS backend over Apple's GameController framework (`objc2`) |
| | | `src/platform/linux/gamepad.rs` | Doubles play (2026-10-06, no task id): a pad whose `/dev/input` node is read-only still works for input, without force feedback |
| | | `src/lib.rs`, `src/platform/default/ff.rs` | B618: no unused-import / unused-variable warnings in the Android and iOS builds |

Not code: the vendored `Cargo.toml.orig`/`Cargo.lock` files are upstream's and never read; `LICENSE-*` files are
kept (gilrs-core's added from upstream's repo, which the crate package lacks).
