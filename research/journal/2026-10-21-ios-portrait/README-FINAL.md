# M144 — iOS app: portrait as well as landscape, held under rotation lock

## What changed
- `tools/ios-ipa.sh` Info.plist: `UISupportedInterfaceOrientations` adds Portrait (iPhone) and Portrait + upside
  down (`~ipad`). Landscape stays first. `tools/release.sh` builds the IPA through `ios-ipa.sh`, so it needed no change.
- `crates/hst-base/src/ios.rs` `orientation` (in `First`, main thread through `NonSendMarker`, wired in `hst`'s
  `main.rs` beside `heartbeat`): the window's view controller is always held to the class it is in (winit
  `ValidOrientations::Landscape` or `::Portrait`). It moves to the other class only when `UIDevice.orientation`
  changes into it **and** the accelerometer (CoreMotion `CMMotionManager`, polled) agrees within 0.5 s (or lies
  flat / has no reading). Then it calls `setNeedsUpdateOfSupportedInterfaceOrientations` too, because winit only
  calls `attemptRotationToDeviceOrientation`, which iOS 16+ ignores.
  - iPhone rotation lock: UIDevice then reports portrait however the phone is held. Turning the lock on in landscape
    shows up as a change to portrait that gravity doesn't agree with, so it's ignored and the game stays landscape.
    While the lock is on, UIDevice doesn't change again, so the game stays frozen even if the phone is turned.
  - iPad rotation lock: the system holds the current orientation itself, and the mask always includes it.
  - Flips inside a class (landscape left ↔ right, and on iPad portrait ↔ upside down) are left to the system.
- Cross-built with `tools/ios-build.sh build --release -p hst --features ship`: no warnings in our crates, and the
  binary links `CoreMotion.framework`.

## Portrait layout (desktop shots, HST_SIZE='450 900', in context/M144/run1/)
- Main menu, title, character select and the match (HUD pinned to the corners, camera fits the court) all lay out
  inside a portrait window, at uniform scale, with nothing cut off. The main menu leaves a lot of empty space above
  and below, but it isn't broken.
- Touch pad (`HST_TOUCH=1`): the face diamond and the stick zone fit. R1 sits on the top-right score's "Games"
  count, and it does the same in landscape (`touch-land.png`), so it isn't caused by portrait. Filed as **B610**.

## Not verified / not 1:1
- Not run on a device (no iPhone or iPad here). Untested: following rotation into portrait and back, holding the
  orientation with rotation lock on, and the iOS 16 `setNeedsUpdateOfSupportedInterfaceOrientations` path. The lock
  handling assumes the documented iPhone behaviour (UIDevice reports portrait while the lock is on) and that iPad's
  lock holds the current orientation; check both on the phone (the log prints `orientation: held …` on each change).
- The 0.5 s window and the 0.5 g "not flat" threshold were chosen, not measured.
- On the original game: none. It's a PS2 game with no orientation, and this is front-end/platform work.
- Portrait layout was checked only on desktop shots at 450×900, not on a phone with real safe-area insets.
