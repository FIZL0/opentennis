# B708: ship a software renderer where the OS has none

B707 added Settings → Display → Rendering → Software (wgpu's CPU adapter). Most Linux systems, SteamOS included, ship
no software Vulkan driver by default, so Software fell back to Hardware there. This task bundles one.

## What landed

- `tools/lavapipe.sh x86_64|aarch64 DIR`: builds Mesa 24.0.9's lavapipe (Vulkan on llvmpipe) into `DIR/vulkan/`:
  `libvulkan_lvp.so` (LLVM 11 linked in statically, ~50 MB, ~15 MB compressed), `lvp_icd.json` (relative
  `library_path`) and the Mesa/LLVM licences. The sources are pinned by sha256 (Mesa, wayland-protocols 1.33, libdrm
  2.4.120 and libxshmfence 1.3.2 built static, meson 1.4.1 + Mako + MarkupSafe as pure Python). x86_64 builds in
  the game's own sniper SDK container (needs GLIBC_2.29, under the game's 2.31 limit); aarch64 in arm64 Debian 11
  under QEMU. Results are cached in `~/.cache/hst-lavapipe/<arch>-<mesa>`, so a release only builds it once.
- `tools/build.sh linux` puts it in `dist/linux/vulkan/`, `linux-aarch64` in `dist/linux-aarch64/OpenTennis/vulkan/`
  (not in the PortMaster zip: there's no window system there for it to present to). `tools/release.sh` refuses to
  release without both, and the tarballs carry them.
- `render_mode::apply`: when Software is picked and `vulkan/lvp_icd.json` sits beside the (symlink-resolved) game
  binary, adds it to `VK_ADD_DRIVER_FILES` (appending to any value already set) before the adapter search. It
  doesn't replace the system's drivers, so Hardware still works if lavapipe can't present. Hardware starts are
  untouched.
- `render_mode::NO_SOFTWARE`: the Settings row's text on iOS/macOS (Metal has no CPU device) and Android (see
  below) once Software is picked; `options.rs` shows it in place of the generic "couldn't start" line.

## Verified

- x86_64 lavapipe from the script, beside a dev build (`target/debug/vulkan/`), `HST_SOFTWARE=1` on this Arch +
  Hyprland (Wayland) desktop with a Radeon: log shows `bundled software driver added`, `software asked for: Vulkan
  llvmpipe (LLVM 11.0.1, 256 bits)`, Bevy's AdapterInfo `driver_info: "Mesa 24.0.9 (LLVM 11.0.1)"`, `rendering:
  software (llvmpipe …)`; the `--shot` title screen (3D court behind it) draws correctly
  (`context/b708/soft.png`).
- `tools/check.sh -p hst-base render_mode` passes.

## Android: no SwiftShader ICD

The task asked for SwiftShader's Vulkan ICD in the APK. Android's Vulkan loader never loads ICD manifests or app
drivers (only the phone's HAL driver; even layers only for debuggable apps), so `VK_ADD_DRIVER_FILES` does nothing
there. Loading `libvk_swiftshader.so` directly (ash `Entry::load_from` + wgpu `Instance::from_hal`) would get a
device, but no window surface: on Android `VK_KHR_android_surface`/swapchain come from the platform loader, not
the driver. A real path is headless SwiftShader + posting each frame with `ANativeWindow_lock`, a much larger
change: follow-up **B709**. The Settings row now says Android has no software renderer the game can use yet.

## Not verified / not 1:1

- Not 1:1: this is renderer/settings work, nothing from the original applies.
- No AppImage or Flatpak exists (the Linux build is the sniper-built tarball, which also serves the Steam Deck);
  lavapipe went into the two tarballs instead.
- Not run on a real Steam Deck, nor in an X11 session or a system without its own lavapipe: only on this Wayland
  desktop. `libvulkan_lvp.so` links the system's libxcb-* (incl. xcb-keysyms), libwayland-client, libudev, libtinfo
  and libstdc++; a system missing one fails to load the driver, and B707's fallback draws with Hardware.
- The aarch64 lavapipe was built (QEMU) but not run on ARM hardware.
- Old Vulkan loaders (before 1.3.207, e.g. Debian 11's) ignore `VK_ADD_DRIVER_FILES`; not worked around
  (`VK_ICD_FILENAMES` would hide the hardware driver Bevy falls back to).
- The self-updater fetches only the bare binary, so updated installs don't get `vulkan/` until re-downloaded:
  follow-up **B710**.
- macOS has no build; its text is there for the cfg only.
