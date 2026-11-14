# B709: Android Software rendering: decided it isn't worth it

B708 found that Android's Vulkan loader only loads the phone's own driver, so SwiftShader can't be added as an ICD.
The way left would be to load `libvk_swiftshader.so` ourselves (ash `Entry::load_from` + wgpu `Instance::from_hal`),
draw with no window surface, and copy every frame into the window with `ANativeWindow_lock`. This task asked to build
that or decide it isn't worth it. **Decision: not worth it.** A phone CPU can't draw this game fast enough.

## Measurement

`tools/bench.sh` (4-bot doubles, court 1, 1280×800, release build), drawing on B708's x86_64 lavapipe (llvmpipe, LLVM 11)
as the only Vulkan driver (`VK_ICD_FILENAMES=…/lvp_icd.json WGPU_BACKEND=vulkan HST_SOFTWARE=1`; the log's
AdapterInfo says `llvmpipe (LLVM 11.0.1, 256 bits)`):

| CPU | fps | ms/frame |
|---|---|---|
| Ryzen 7 9800X3D, all 16 threads | 9.4–9.5 | ~106 |
| same, `HST_SLOW=1` (2 cores, 120 % quota) | 0.9–1.1 | ~1000 |

On the fastest desktop CPU there is, the match runs at under a sixth of its 60 fps. A phone has 2–4 big cores that are
each several times slower than these, and they throttle under constant load, so expect around 1–3 fps. That's before
the extra cost of copying every frame back from the CPU renderer and into the window. SwiftShader (LLVM) and
llvmpipe are about as fast as each other; we can't measure SwiftShader directly here (see below).

## Cost, had it gone ahead

- SwiftShader built for Android arm64 (+ x86_64 for the emulator) with the NDK. arm64 needs SwiftShader's LLVM
  backend (Subzero has no arm64), which is a long C++ build to pin and cache in `tools/apk.sh`, and adds ~15–25 MB to
  an APK that's already 134 MB.
- A present path that Bevy and wgpu don't have: a fake swapchain in our wgpu-hal fork (or a headless window target in
  bevy_render) that reads each frame back and copies it into `ANativeWindow_lock`'s buffer, handling stride, format
  and rotation.
- Only checkable on the emulator: the phones it might help are ones whose GPU driver is broken, and wgpu already
  falls back from Vulkan to GL ES on those.

## What changed

- `render_mode::NO_SOFTWARE` (Android): the Settings text no longer says "yet". It now says a phone's processor is
  too slow to draw the game by itself, so Software draws with Hardware. The doc comment records the measurement.

## Not verified / not 1:1

- Not 1:1: renderer/settings work; nothing from the original applies.
- Phone speed is estimated from the desktop numbers, not measured: no Android build of SwiftShader exists here, and
  building one was the cost being weighed.
- The Android SDK emulator's own host SwiftShader (`emulator/lib64/vulkan`, Subzero, Vulkan 1.0.5) dumped core
  right after Bevy created the window, so SwiftShader itself wasn't benched. llvmpipe was used in its place.
- The Settings text change wasn't looked at on a device or emulator. It's a string swap in the row's existing text.
