# B706 — GitHub #4: players invisible on an Adreno 730 (Android, Vulkan)

## Evidence
- The reporter's `hst-log.txt` (Lenovo TB320FC, "Adreno (TM) 730", Qualcomm Vulkan driver of 2025-02-14) has
  `GPU preprocessing is not supported on this device. Falling back to CPU preprocessing.` Bevy 0.19 keeps GPU mesh
  preprocessing off for Adreno ≤ 730 (except the 720) and Mali drivers < 48 (`is_non_supported_android_device`).
  The Adreno 732 that works isn't on that list, so it runs GPU preprocessing. That's the one rendering difference
  between the two phones.
- The reporter's match screenshot shows the players not culled but **exploded**: streaks of their textures fanned
  across the net. Racquets (rigid meshes on the hand bone), the ball and the court draw right. Character-select
  previews of the skinned face come out empty (`the preview drew nothing`) or as junk shapes in the face grid. So
  skinning reads the wrong matrices, while static meshes reading the same per-mesh storage buffer are fine.
- Desktop (RADV) with the CPU path forced (`HST_CPU_PREPROCESS=1`, new) draws players correctly, with storage skins as
  before. So the CPU path itself is fine. What breaks is this driver on the CPU path with skins read from one big
  storage buffer at `mesh[i].current_skin_index + joint`.

## Fix (shared path, not a device check)
Where Bevy's own driver list sends a GPU to CPU preprocessing, skins and morph weights now use the layout of the path
built for CPU batching (WebGL2's): joint matrices and morph weights in uniform buffers at a dynamic offset, morph
targets in a 3D texture. Per-mesh data stays in storage buffers. Patched in third_party (README rows):
`bevy_render::gpu_preprocessing_blocked` (Bevy's list + switches, also used by `GpuPreprocessingSupport`),
`choose_skin_buffers` (decided once when the renderer starts, before any layout), `skins_use_uniform_buffers`.
bevy_pbr's `skins_use_uniform_buffers` and bevy_render's morph-target allocator ask it.

Switches for the device (`debug.txt` in the data folder, `mod_debug.rs`), so the reporter can test without a new build:
`uniform_skins = on|off` overrides the skin layout, and `gpu_preprocess = on|off` skips Bevy's list or forces the CPU
path. `HST_CPU_PREPROCESS=1` does the latter on desktop.

## Checked
- Desktop, `HST_CPU_PREPROCESS=1`: the log says `skins and morph weights in uniform buffers`, and a doubles match
  (`--stage 1 --play`, shots in `context/b706/cpu4.png`) looks the same as the GPU path. The select's live previews
  (`HST_MENU=roster1 HST_PREVIEW=live`) draw.
- Desktop default: still `GPU preprocessing is fully supported`, storage skins, no uniform line.
- `tools/patches.py` passes. The `mod_debug` test covers the two new keys.

## Not verified / not 1:1
- **Not tested on an Adreno 730.** No such device here, and the bug didn't reproduce on desktop with the same Bevy
  path, so the fix rests on the evidence above, not on a device run. I asked the reporter on #4 to try the next
  release (and the `debug.txt` switches if it isn't fixed). Follow-up B706a closes #4 once they confirm.
- In the uniform layout a skin holds at most 256 joints (Bevy's `MAX_JOINTS`, the same cap as the WebGL2 build). Mod
  rigs bigger than that would skin wrong on these GPUs only. Disc characters are far below it.
- Rendering only: no sim, RNG or timing code touched.
