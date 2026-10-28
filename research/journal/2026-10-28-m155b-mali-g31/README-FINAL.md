# M155b: OpenTennis on the Mali G31 blob (RG34XX SP, muOS 2606)

Interactive session against the device (root@192.168.0.225), PortMaster port in `/mnt/mmc/ports/opentennis`,
launcher `/mnt/mmc/ROMS/Ports/OpenTennis.sh`. Device facts: Allwinner H700, Mali-G31 MP2, blob
`OpenGL ES 3.2 v1.r20p0`, EGL 1.4 on fbdev (`/dev/fb0` 720x480, virtual 720x960, 32 bpp), kernel 4.9.170, no `/dev/dri`.

## What was wrong, in the order it showed

1. **No adapter** (`Unable to find a GPU!`). wgpu-hal's EGL finds a window config but none `NATIVE_RENDERABLE`, and
   off Android/Windows/OHOS it then marks every surface unpresentable. Fix (vendored `wgpu-hal` 29.0.4,
   `create_surface`): a surface on an `OhosNdk` handle (our fbdev window) is presentable.
2. **Compute/storage**: Bevy's compute pipelines want 256-invocation groups (Mali: 128) and a storage buffer in the
   prepass vertex stage (no `VERTEX_STORAGE`). Fix: `graphics::webgl2()` is true on the framebuffer path, so the game
   runs on WebGL2's limits (no compute, no storage buffers, CPU batching), as the web build does.
3. **`noperspective`** (the GS fog varying) isn't in GLSL ES: `GS_NO_LINEAR` on ARM Linux (the F·w/w trick).
4. **One-layer shadow array** viewed as `D2Array` (wgpu-hal: "assumed … D2"): the directional/spot shadow map gets
   two layers on ARM Linux (vendored `bevy_pbr` `light.rs`).
5. **Black screen** with everything "working": wgpu asked for `EGL_SINGLE_BUFFER` on Linux, and the Mali fbdev display
   never flips to a single-buffered surface. Proven with a C smoke test (`eglCreateWindowSurface` on `{720,480}`,
   back buffer: red/green/blue clears reached `/dev/fb0`). Fix: `BACK_BUFFER` for the `OhosNdk` window.
6. **Black at 4x MSAA**: MSAA resolves never reached the screen; `webgl2()` makes `msaa_drawn()` 1 (Options greys it).
7. **Guard band**: without `GS_GUARD_DISCARD` the NaN positions stalled the blob (0.3 fps, seconds-long frames, black);
   the fragment-stage discard (B485, the web's) on ARM Linux too. Checked both ways.
8. **Pads**: muOS grabs the real pad; the game sees the virtual `muOS-Keys`. PortMaster's mapping failed to parse
   (`volumedown`/`volumeup` unknown to gilrs → whole mapping dropped), muOS numbers buttons in key-code order (volume
   keys first; gilrs put them last → two off), and muOS's default "retro" db maps by position (A read as B). Fixes:
   the launcher strips the volume entries, takes the `modern.txt` line on muOS, sets `GILRS_SDL_ASCENDING=1`, which
   the vendored gilrs-core honours in `find_buttons`. Checked by writing evdev events to `/dev/input/event3`
   (A→South/ok, B→East/back, X→West, Y→North, L/R, L2/R2, Select, Start, Menu→Mode, L3/R3).

## Speed

Auto picks Lowest on the framebuffer path (50% scale, no shadows, no MSAA, quarter meshes). Menu ~25 fps, a
singles/doubles match ~25-27 fps (`HST_FPS`: game ~26 ms main thread incl. sim ~6-9 ms, render thread ~25 ms of GL
calls). CPU-bound: the CPU already runs 1.8 GHz under schedutil, and pinning the GPU at 816 MHz changed nothing.
Default graphics draw correctly at ~16 fps. The sim stays fixed 60 Hz.

## Device checks done

Title → Main → Local → character select → ready → Confirm → a singles match on grass, served, rallied (screens via
`/dev/fb0` at the panned page); Start+Select quits through gptokeyb. `tools/build.sh linux-aarch64` with
`HST_FEATURES=` (empty) builds without `ship` for device sessions.

## Open

- ~25 fps: more would need fewer draws on the render thread (wgpu-GL per-draw cost) and less main-thread work.
- The pad's analog sticks under muOS-Keys weren't exercised (d-pad hat and buttons were).
