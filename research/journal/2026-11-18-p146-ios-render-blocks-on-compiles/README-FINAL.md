# P146: iOS render thread blocks on async pipeline compiles

## What the render thread waited on

hst-log 6 (iPhone 18 Pro, 1.7.2): `queue+prepare` 443.6 ms (menu load, ready +9 inside the frame) and 556.7 ms
(match load, ready +12 inside the frame). Pipelines finishing *inside* one render frame's queue+prepare means the
render thread itself waited on the async pool.

Not a wgpu lock: wgpu-hal 30's Metal device has no device mutex any more, and wgpu-core 30's
`create_render_pipeline` takes no device-wide lock across the hal call.

It is Bevy's upscaling system (`bevy_core_pipeline` `prepare_view_upscaling_pipelines`, in `RenderSystems::Prepare`,
so inside our queue+prepare span). When a view gets a blit key it hasn't had (a new camera / output mode at menu
or match load), it specializes the blit pipeline and calls `PipelineCache::block_on_render_pipeline`, which:

1. saw the new id wasn't in `pipelines` yet and ran a full `process_queue()`: every pipeline queued that frame
   was spawned onto the async compute pool, the blit one somewhere among them;
2. `block_on` the blit's task, which sits behind the others on the pool, and every compile task takes the shared
   `shader_cache` mutex for its shader-module step, so they run one at a time.

So the frame waited for most of the frame's new pipelines. On Metal (naga → MSL → library per pipeline) that's
tens of ms each.

## Fix (vendored bevy_render `pipeline_cache.rs`)

`block_on_render_pipeline` now moves new pipelines into the cache without starting them (`take_new_pipelines`),
and compiles only the wanted pipeline in place on the calling thread (sync, ahead of the queue). If that pipeline
was already spawned and isn't done, its task is dropped (cancelled if not started) and it compiles in place; the
shader modules are shared through the shader cache either way. The rest compile on the pool from the next
`process_queue` as before.

## Measured (Linux, Vulkan, `HST_PREWARM=0 MESA_SHADER_CACHE_DISABLE=true HST_FPS=1`, start-up to title)

Temporary timing around `block_on_render_pipeline` (not committed), two runs each:

| call | before | after |
|---|---|---|
| 1st (new) | 6.7 / 7.4 ms | 0.35 / 0.34 ms |
| 3rd (new) | 24.4 / 29.0 ms | 0.11 / 0.10 ms |

Same mechanism; on desktop Vulkan the compiles are short, so the desktop never showed it as a >50 ms stall.
Title screen renders as before (shot checked).

## Not verified / not 1:1

- Not run on an iPhone (no device here): that the 440-560 ms queue+prepare frames are gone on iOS needs a new
  hst-log from the phone with `HST_PREWARM=0` (or a fresh install) at menu and match load. If a long
  queue+prepare remains with `ready +N` inside the frame, something else in Prepare waits on a pipeline.
- The in-place blit compile still takes the shader-cache mutex, so it can wait for one pool task's shader-module
  step (not for whole queued compiles).
- Rendering/performance only; no gameplay, sim RNG or match timing touched.
