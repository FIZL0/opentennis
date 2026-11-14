# B707 — Display → Rendering: Hardware or Software, every build

## What landed
- `crates/hst-base/src/render_mode.rs` (new): `rendering = hardware|software` in settings.txt, read before the app
  starts (`hst` main.rs → `render_mode::apply`).
  - Native: Software enumerates the adapters of the backends Bevy would use and names the first CPU one
    (`DeviceType::Cpu`, or llvmpipe/lavapipe/softpipe/SwiftShader/"Basic Render" in its name) as `adapter_name`.
    Bevy's own `find_adapter_by_name` then takes it; if the surface rejects it, Bevy's usual pick (hardware) runs.
    None found → hardware, `MISSING` set (the row's bracket and hint say so).
  - Crash guard: a Software start writes `rendering-try.txt` (`trying`) beside settings.txt; 30 frames drawn on
    Software remove it. Found at start → `failed`, Software skipped (hardware) until the menu picks Software again.
    Without it a driver that crashes at the first frame would lock the player out of the menu.
  - Web: index.html reads `rendering`; Software asks `navigator.gpu.requestAdapter({forceFallbackAdapter:true})`,
    and if the browser has one loads the WebGPU build on it (`window.hstSoftware` → `force_fallback_adapter`).
    WebGL2 can't be asked to draw on the CPU: then MISSING. `hstRendering` reloads the page on a change.
  - Restart after the popup's Switch: Unix exec of the same exe+args, Windows spawn+exit, Android the activity's
    restart, browser reload; iOS can't restart itself, so its popup says "It changes the next time you open the game".
  - `HST_SOFTWARE=1` forces Software for a run (test windows, `HST_BG`, otherwise ignore settings.txt's mode).
- Settings → Display: `Rendering` row under Display (desktop/phones) or Renderer (web). ✕ opens a "Switch to
  Software? / The game restarts to change it." popup (reuses B327's `switch` popup, `MODE_SWITCH` bit); ◀ ▶ do
  nothing, as on Renderer (no arrows drawn). Value shows the running mode in brackets when it differs
  ("Software (Hardware)"). Hint is the task's text.
- `graphics::SOFTWARE` detection now shares `render_mode::is_software` and logs how Software went
  (`render_mode::report`, once the log is up).
- Tests: `options::tests::rendering_asks_first` (popup flow, settings.txt round trip, popup drawn at 3 aspects),
  `render_mode::tests::settings_word`; `every_page` skips Rendering (it asks instead of flipping).

## Checked
- `--shot` of the Display page and of the popup (context/b707/). Row fits; popup laid out like Renderer's.
- This machine has no lavapipe: `HST_SOFTWARE=1` → hardware, warning logged, row's hint switches.
- Chromium's SwiftShader Vulkan ICD (`VK_ADD_DRIVER_FILES=/usr/lib/chromium/vk_swiftshader_icd.json`) is picked
  (`AdapterInfo … device_type: Cpu`), proving the selection path; that build is headless (no present modes; on
  Wayland it segfaults in its WSI), so the first frame panics. The guard then went trying → failed and the next
  starts came up on hardware; a Hardware start leaves `failed` alone.
- `tools/webcheck.sh` (wasm32, ship) type-checks.

## Not verified / not 1:1
- Software actually drawing a match: no working software Vulkan driver here (no lavapipe installed, Chromium's
  SwiftShader can't present). The guard's success path (marker removed after 30 Software frames) is untested live.
- Android build not type-checked here (needs the NDK C toolchain; one cfg-gated call to the existing `android::restart`); Windows hst-base checks clean.
- Windows WARP, Android, iOS, macOS, handhelds and the browser's fallback adapter: not run. Expected: Windows finds
  WARP (DX12, "Microsoft Basic Render Driver"); macOS/iOS have no CPU Metal device → MISSING; Android usually none;
  Chrome hands out a fallback WebGPU adapter only on some setups.
- The restart itself (exec/spawn/activity restart/reload) wasn't driven through the menu.
- Not a gameplay port: front-end setting, nothing to match against the original.
- Follow-up B708: ship lavapipe/SwiftShader where the OS has no software driver.
