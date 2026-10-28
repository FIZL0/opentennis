# B694 — iOS rotation lock: hold landscape, island on the left

User 2026-10-28: with an iPhone's rotation lock on, the game should go to landscape with the Dynamic Island /
camera on the left (UIInterfaceOrientationLandscapeRight), not hold its last orientation (M144). Lock off: follow the
device as before.

## What changed
- `third_party/winit`: a `ValidOrientations::LandscapeRight` variant mapped to
  `UIInterfaceOrientationMask::LandscapeRight` (home side right, camera left); listed in `third_party/README.md`.
- `crates/hst-base/src/ios.rs` `orientation`: the held state is now Landscape / Portrait / Locked. Rotation lock is
  detected as UIDevice reporting portrait while the accelerometer says the phone is sideways for ≥ 1 s (M144 found
  the lock makes UIDevice report portrait however the phone is held; a normal turn reports within a moment, so 1 s
  separates the two). Then the controller is held to LandscapeRight. It leaves Locked only on a device-reported turn
  the accelerometer agrees with (lock off and the phone turned), as before for portrait ↔ landscape.
- `tools/ios-ipa.sh`: Info.plist comment updated (orientation keys unchanged: LandscapeRight is already allowed).
- Checked: `tools/ios-build.sh check -p hst-base` builds clean.

## Not verified / not 1:1
- Not run on an iPhone (no device in an unattended run): needs the user to try the next IPA — lock on held
  sideways (either way) → turns to island-left within ~1 s; lock on held upright → stays as it was until the phone is
  turned sideways once; lock off → follows the device.
- Lock turned on while the phone is held upright can't be seen (UIDevice already says portrait, gravity agrees), so
  the game only goes to landscape once the phone is turned sideways. Likewise, lock turned off while upright and
  in Locked keeps island-left until the phone is turned (UIDevice reports no change). No API reports the lock state.
- The portrait-under-lock UIDevice behaviour is taken from M144's finding, not re-checked here.
- Original-game comparison: not applicable (platform shell, not gameplay).
