# M155: muOS / framebuffer handhelds (no Wayland/X)

The M154 aarch64 build panicked in bevy_winit on muOS: there is no display server, and the only GPU path is the Mali
blob's EGL on fbdev (`/dev/fb0`).

## What landed

- `crates/hst-base/src/fbdev.rs` (ARM Linux only): stands in for winit when there's no Wayland/X (`WAYLAND_DISPLAY`,
  `WAYLAND_SOCKET` and `DISPLAY` all unset) and `/dev/fb0` exists. `HST_FBDEV=1|0` forces it either way. Shipped builds
  keep this one `HST_*` variable (`main.rs` `dev_env` KEEP list); they drop the rest.
  - The window is the Mali fbdev EGL's native window, `{ u16 width, u16 height }` (the blob's `fbdev_window`), sized
    from `FBIOGET_VSCREENINFO` (fallback: `/sys/class/graphics/fb0/modes`).
  - Handed to wgpu as an **OhosNdk** raw-window-handle with an `Ohos` display handle. I read wgpu-hal 29's EGL code to
    pick this. With no Wayland/X display handle it falls back to `eglGetDisplay(EGL_DEFAULT_DISPLAY)` (the Mali
    surfaceless check only applies to Mesa). Its `create_surface` accepts OhosNdk on any OS, and `configure` passes
    that handle's pointer to `eglCreateWindowSurface` unchanged. Nothing in wgpu needed patching.
  - The primary window's size is set to the framebuffer at scale 1. The handles go into the window's
    `RawHandleWrapperHolder`, which M154's bevy_render patch waits on before creating the instance. One
    `WindowCreated` message is written.
  - Runner: wait for `PluginsState::Ready` (ticking the main-thread task pools, as winit's runner does), then
    `finish`/`cleanup`, then `app.update()` until `should_exit`. Frames are paced by the swap.
- `main.rs`: on ARM Linux, `fbdev::wanted()` disables `WinitPlugin` and adds `fbdev::plugin`. Everywhere else nothing
  changes; it's all compiled out.
- I didn't use SDL2. It isn't needed for the window, and the pads already work through gilrs, so no SDL dependency was
  added:
  - The launcher sources PortMaster's `control.txt` where present and exports `SDL_GAMECONTROLLERCONFIG`, which gilrs
    reads for the built-in controls' mapping.
  - With no udev daemon (`/run/udev/control` and `/run/udev/data` both missing), the launcher sets
    `GILRS_DISABLE_UDEV=1`. gilrs then scans `/dev/input` itself (inotify). Without that, its udev enumeration would
    find no `ID_INPUT_JOYSTICK` property and no pads.
- `crates/hst/linux/OpenTennis.sh` (now `#!/bin/bash`, as PortMaster ports are) finds the game in either layout:
  - beside the script (Knulli/ROCKNIX `roms/ports/OpenTennis/`);
  - PortMaster's split: the script in `ROMS/Ports/`, the folder in `/$directory/ports/OpenTennis`, `/mnt/mmc/ports/OpenTennis`
    or `/mnt/sdcard/ports/OpenTennis`, or `OpenTennis/` beside the script.

  The ISO lookup also tries `/mnt/*/ROMS/PS2/*Hot*Shots*Tennis*.iso` (muOS's roms folder).

## Checked

- Built: `cargo check -p hst --target aarch64-unknown-linux-gnu` in the build container is clean, and
  `tools/build.sh linux-aarch64` builds (needs GLIBC_2.30, as before).
- Arm64 Debian 11 under qemu-user, with no `DISPLAY`/`WAYLAND_*`, a placeholder `/dev/fb0` and the disc pack:
  - No winit panic. `fbdev` logs "no display server: drawing to /dev/fb0" (the size came from the host's sysfs mode).
  - The runner starts, and the renderer thread gets the handles and creates the wgpu instance.
  - It then stops with "Unable to find a GPU!", as expected there: Mesa offers `EGL_MESA_platform_surfaceless`, so
    wgpu takes its surfaceless display, which can't present to a window. The Mali blob has no such extension and
    takes the default (fbdev) display instead. Logs: `context/m155/run3.log`.
- Before the fix (same container, no display): the reported panic, "neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor
  DISPLAY is set".
- The launcher was tested with a stub binary:
  - in-folder layout;
  - PortMaster split layout found via `$directory`, with a fake `control.txt` whose `get_controls` sets
    `sdl_controllerconfig` (exported as `SDL_GAMECONTROLLERCONFIG`);
  - the `OpenTennis/` subfolder beside the script, with args passed through;
  - game not found (the log says where it looked).

## Not verified / not 1:1

- **Not run on the RG34XX SP / muOS.** There's no device here, and no emulator has the Mali fbdev blob. Untested:
  - that the blob's `eglCreateWindowSurface` takes this native window (the `fbdev_window` layout is the Mali fbdev
    convention; some blobs also take NULL);
  - that `eglGetProcAddress` gives wgpu the core GLES functions;
  - which EGL config wgpu picks for the framebuffer's pixel format;
  - page flipping and vsync on fb0;
  - frame rate;
  - that wgpu's GL ES path renders on the G31 blob (M154 only ran Mesa llvmpipe).
- **Display rotation.** The framebuffer's size is used as is. A panel that muOS rotates in software would draw
  sideways; the RG34XX SP's 720x480 landscape fb should be fine.
- **Pads.** gilrs + PortMaster's `SDL_GAMECONTROLLERCONFIG` + the udev fallback were not tried on the device. Whether
  muOS's Ports start actually exports a PortMaster `control.txt` at the paths the launcher looks in
  (`/opt/system/Tools/PortMaster`, `/opt/tools/PortMaster`, `$XDG_DATA_HOME/PortMaster`, `/roms/ports/PortMaster`,
  the standard port header's list) comes from PortMaster's port template, not from a muOS device.
  - Without a mapping, gilrs falls back to evdev's own button codes.
- **No hotkey exit (gptokeyb).** The game is left through its own Quit.
- **No input or focus events from the system.** No keyboard, no window resize, no suspend/resume lifecycle. A
  physical keyboard on the device is ignored.
