# M154 — Linux aarch64 release build

Target, per the task line as updated on main mid-task (37c37659): ARM handhelds running ROCKNIX, first the Anbernic
RG34XX SP (H700, Mali G31 on Panfrost/Mesa), wgpu on GL ES unless PanVK Vulkan works, glibc no newer than ROCKNIX's,
a ports launcher for ROCKNIX's `roms/ports/`. (The task branch started from the older Knulli wording; nothing in the
build depends on which of the two it is.)

## What landed

- `tools/linux-aarch64.sh setup|build`: the ARM Linux build, cross-compiled (`aarch64-unknown-linux-gnu`) inside a
  Debian 11 container, so the binary needs glibc ≤ 2.31 (it needs GLIBC_2.30; the script checks, like
  `tools/build.sh linux` does for sniper). ROCKNIX ships a far newer glibc. `setup` installs the cross gcc and the
  arm64 ALSA/udev dev packages (the only libraries the game links: `readelf -d` shows libdl, libudev, libasound,
  libgcc_s, libpthread, libm, libc; EGL, Wayland and X11 are loaded at run time). Debian 11 is past its end of life
  and its security pool already 404s on deb.debian.org, so `setup` points apt at archive.debian.org, where it stays
  for good.
- `tools/build.sh linux-aarch64` (and `all`): runs that in docker (`--platform linux/amd64`: the cross compiler is
  x86); the set-up container is committed once as the local image `hst-linux-aarch64`; Rust/cargo cache in
  `~/.cache/hst-linux-aarch64`, build in `target/linux-aarch64`. Output: `dist/linux-aarch64/OpenTennis/` =
  `opentennis`, `OpenTennis.sh`, `opentennis.png`, `opentennis.desktop`.
- `crates/hst/linux/OpenTennis.sh`: the Ports launcher. cds into its own folder, logs to `OpenTennis.log` there (a
  Ports start has no terminal), sets `WGPU_BACKEND=gl` unless already set (`WGPU_BACKEND=vulkan` tries PanVK), and
  with no disc imported yet passes `--import` the first `.iso` in its folder or a `*Hot*Shots*Tennis*.iso` in
  `roms/ps2/` (`../../ps2` from `roms/ports/OpenTennis/`), since these systems have no zenity/kdialog for the game's
  picker. Checked with a stub binary: no ISO (log says where to put one), ISO in roms/ps2, ISO beside it (wins),
  imported pack (no args), args passed through, started through a symlink.
- Renderer on GL ES. Three things stood between the build and a picture:
  1. wgpu's `gles` feature was not compiled into native Linux builds at all (Vulkan only). `hst-base` turns it on for
     `cfg(all(target_os = "linux", target_arch = "aarch64"))` only; x86_64 builds are unchanged.
  2. With GL picked, creating the window's surface panicked (`Fallback system failed to choose present mode …
     Options: []`): wgpu 29's EGL instance only presents to the display it was made on, and Bevy makes the instance
     with `display: None`, synchronously in `RenderPlugin::build`, before winit has a window. Vendored bevy_render
     patch (linux-aarch64 only): the renderer is created on a thread that waits for the primary window's handle
     (bevy_winit makes the window in `resumed` while plugins aren't ready yet, and `RenderPlugin::ready` holds the app
     until the resources land, the same path the browser build takes), and the instance gets that window's display.
  3. Bevy's SSAO plugin builds its compute pipelines up front wherever the GPU has ≥ 5 storage textures; naga's GLSL
     for its depth-prepass `textureGather` is a shadow-sampler gather with no GLSL overload, and the failed pipeline
     quits the app. Vendored bevy_pbr patch: no SSAO on the GL backend (a runtime check; the game uses no SSAO).
  4. A match then panicked "Could not lock adapter context": Bevy compiles pipelines on several pool threads, GL
     serialises them behind wgpu's context lock (6 s timeout), and slow compiles timed the waiters out. `main.rs`:
     ARM Linux compiles pipelines on the render thread (`synchronous_pipeline_compilation`, the macOS/browser path
     with P47's time budget).
  5. ~3700 "wgpu-hal heuristics assumed … Cube rather than CubeArray" errors: GL can't bind Bevy's point-light
     shadow cube-array view. ARM Linux takes the iOS simulator's path (a Cube view, `NO_CUBE_ARRAY_TEXTURES_SUPPORT`),
     a compile-time cfg in bevy_pbr's `light.rs`/`mesh_view_bindings.rs` and bevy_render's `pipeline_cache.rs`.
  All vendored patches are in `third_party/README.md`; `tools/patches.py` passes. Every change is gated to
  linux-aarch64 at compile time except the SSAO one (runtime: GL backend only), so other builds are unchanged.
- Updater: `update.rs` names `opentennis-linux-aarch64` as this platform's bare binary.
- `tools/release.sh`: ships `opentennis-linux-aarch64.tar.gz` (the `OpenTennis/` folder, so it unpacks straight
  into `roms/ports/`) and the bare `opentennis-linux-aarch64`; both land in SHA256SUMS through the existing
  `opentennis-*` glob; a row in the release notes' download table.
- `pages.yml`: a `linux-aarch64` job (debian:bullseye container, same script) whose tarball goes on the beta release;
  the beta waits for it like the others.

## Checked

- `tools/build.sh linux-aarch64` builds from scratch (image set-up included).
- The binary runs on arm64 Debian 11 (glibc 2.31) under qemu-user: `docker run --platform linux/arm64`, Xvfb, Mesa
  llvmpipe, `WGPU_BACKEND=gl`, the real ISO, `--shot`: the title screen renders correctly (court, logo, text);
  frame in `context/m154/run4/menu.png`. `--play --shot-at 8`: players, rackets, HUD and legend render, and the
  frame matches the native x86 build's (`context/m154/run7/match.png` vs `context/m154/host-match.png`; plain
  `--play` draws no court on either), with no render errors left in the log.
- `cargo check -p hst` (host) passes; full tests in `tools/done.sh`.

## Not verified / not 1:1

- Not run on an ARM device: no RG34XX SP / ROCKNIX here. The GL path was only exercised on Mesa llvmpipe (desktop GL
  4.5 through EGL on X11) under emulation, not Panfrost GL ES 3.1 on a Mali G31, not Wayland/Sway (ROCKNIX's
  compositor; the display-handle fix covers both, only X11 was run). Untested on the device: which Bevy pipelines
  Panfrost's limits accept, frame rate at 640x480, whether the built-in controls show up as a gilrs gamepad, PanVK.
- The `pages.yml` job has not run (it runs on the next push to main).
- `.chd` PS2 images (common on these systems) can't be read; the launcher's log says to use an `.iso`.
