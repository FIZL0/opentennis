# P137 — Phone battery

An iPhone 17 Pro at 120 fps drained 5% every 4 min and got hot (user 2026-11-16). Four parts:

1. **Low Power Mode / battery saver caps at 60** — `graphics::LOW_POWER` (static) and `Graphics::fps_drawn()`:
   the limit actually drawn is at most 60 while it's on; the setting itself is untouched. `limit` (frame pacing) and
   the iOS display-link rate (`ios::frame_rate`, now re-sent whenever the drawn limit changes, not only when
   `Graphics` changes) use it. iOS: `ios::watch_low_power` reads `NSProcessInfo.isLowPowerModeEnabled` at start and
   again on `NSProcessInfoPowerStateDidChangeNotification`. Android: `android::watch_power_save` reads
   `PowerManager.isPowerSaveMode()` every 3 s (a Last system on the base app, added in `graphics::plugin`).
2. **Menus and the pause screen draw only while something moves** — new `hst_base::calm`. A screen calls
   `calm::want()` every frame (main menu mode: always; the pause screen: when it's open, offline, and not closing).
   After 0.5 s of nothing moving, `WinitSettings.focused_mode` goes to `reactive(1/30 s)`: it waits for input and
   wakes 30 times a second to read the pads (gilrs input doesn't wake winit) and keep the ambient loops going.
   Any move puts it back to `Continuous` that same frame; a mode that stops asking (a match) is back to full rate the
   next frame. Not in `--shot`/`HST_BG` runs (they count frames) unless `HST_CALM=1`, which also logs each switch
   and, once a second, what is still moving.
   - Change marks were useless as they stand: the menu's draw rewrites all ~600 quad nodes (Node, UiTransform,
     ImageNode, Visibility) and the menu camera's Transform every frame. So `calm` compares values: Bevy's computed
     UI values (ComputedNode, UiGlobalTransform, InheritedVisibility; Bevy only marks those when they differ), a hash
     of the shown ImageNodes and outline colours, and each changed GlobalTransform against the last one seen (within
     1e-4, skipping NaN ones: some hidden piece in a match sits at NaN; something eases toward a target forever).
   - Ambient loops: the menus' wagging hand (its render layer, `calm::ambient_layer`) and the home screen's breathing
     selected row (its quads marked `calm::Ambient` by `home::breath_marks`). They play on at 30 fps while still.
3. **Battery note** — `graphics::battery_note(fps)`: on iOS/Android, Frame Limit above 60 (or Uncapped) shows
   "Smoother, but uses about twice the battery and runs the phone warmer." as the row's note, and the dropdown's
   choices above 60 carry the hint "smoother, ~2x battery, warmer". Desktop shows nothing new.
4. **No preset/default above 60 on a phone** — `PHONE_OS` const; High's and Default's fps is `PHONE_FPS` (60 on
   iOS/Android, 0 = uncapped elsewhere). `upgrade()` brings a phone's saved uncapped High/Default back to 60 (they
   were saved that way before); a player's own 90/120 stays. Phone/Low/Lowest/Potato were already 60; Auto starts
   from Phone and `cheaper` never raises fps. The sim is untouched (fixed 60 Hz), so replays stay exact.

## Checked

- `cargo check` for aarch64-apple-ios (`tools/ios-build.sh check -p hst`) and aarch64-linux-android (`-p hst --example
  hst --features android,ship`): clean.
- Desktop, debug build, `HST_CALM=1` in shot windows (logs in `context/p137/`):
  - Main menu home: still at 1.35 s, 30 fps (p50 33.3 ms over 151 frames); `HST_KEYS` ArrowDown at 4 s and 7 s → full
    rate at 4.02/7.03, still again at 4.52/7.72.
  - Settings → Graphics: still at 1.37 s, 30 fps.
  - `--play`, Escape at 12 s: pause still at 12.78; ArrowDown at 20 → full, still at 20.52; Escape at 24 → full rate
    and no request after it (play never slows).
  - The title screen (court behind it moving) and character select (3D previews) stay at full rate, as they move.
- Tests: `calm::tests` (settle/wake, ambient layers), `graphics::tests::phones_stay_at_60`; hst options/home/menu.

## Not verified / not 1:1

- No phone here: Low Power Mode / battery saver detection, the iOS notification, the display link following the cap,
  winit's reactive mode on iOS/Android and the battery note's look were only compile-checked for those targets, not
  run on a device. The battery saving itself (and the "about twice the battery" wording) is not measured.
- The idle rate (30 fps) and settle time (0.5 s) are our picks; pad presses while still can take up to 33 ms to be
  read (key/touch input wakes at once).
- Front-end pacing is ours to design (PLAN.md); nothing here touches the sim or match timing.
