# M192: phone stays awake through a match

**Problem:** on iPhone a paused match (or one played on a controller) gets no touches, so iOS's idle timer dimmed and
locked the screen. Android had no keep-screen-on either (no `FLAG_KEEP_SCREEN_ON` anywhere in the activity).

**Change:**
- `hst-base/src/ios.rs` `keep_awake(on)`: sets `UIApplication.idleTimerDisabled` on the main queue. The first call
  adds two notification observers: entering the background turns the idle timer back on, coming to the front again
  re-applies what the match wants.
- `hst-base/src/android.rs` `keep_awake(on)` → new `HstActivity.keepAwake(boolean)` (adds/clears
  `FLAG_KEEP_SCREEN_ON` on the UI thread). The flag only holds while the window is in front, so the background needs
  nothing more. R8 keeps it (`-keep class io.github.fizl0.hst.** { *; }`).
- `hst-play/src/play/front_end.rs`: the match plugin inserts a `KeepAwake` resource (turns it on); `mode.rs`'s
  teardown drops the match's resources when the menu (or the next mode) starts, and its `Drop` turns it off. So it
  covers the whole match, paused included, every kind of match the play plugin builds (solo, local, online), and a
  match → match switch goes off-then-on in order on the same main queue. Desktop: a no-op.

**Checked:** `cargo check` for desktop, `tools/ios-build.sh check -p hst-play` (aarch64-apple-ios) and
`cargo check -p hst --example hst --features android,ship --target aarch64-linux-android` all build clean, no
warnings; `javac -Xlint:all -Werror` on the activity passes.

## Not verified / not 1:1
- Not run on a real iPhone or an Android device/emulator: unattended run, no phone attached, and the user tests UI
  live. Needs a look on the phone: pause a match, leave it ~past the auto-lock time, the screen should stay on; back in
  the menu it should lock as usual.
- Not a gameplay port: the original (a console game) has no idle timer, so nothing to compare against.
