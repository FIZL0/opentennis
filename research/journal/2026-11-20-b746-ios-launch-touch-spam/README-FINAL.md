# B746 — iOS: touches spammed at launch leave the on-screen pad dead

## What the phone showed (pulled over USB while the app was still open)

- `afcclient --container io.github.fizl0.hst ls Documents` works on the sideloaded app (UIFileSharingEnabled);
  `--documents` gives Permission denied. Pulled `hst-log.txt` (no `hst-log-previous.txt` on the phone),
  `controls.txt`, `hints.txt`, `pipelines.json` into `context/b746/`. `idevicesyslog` only gives live lines (the app
  was already in the background, so nothing came); `idevicescreenshot` needs the developer disk image (not mounted);
  `idevicecrashreport` had only an old crash (2026-11-15) from before this.
- v1.8.3 build 5164. **The disc image was on the phone** (`Documents/Hot Shots Tennis (USA).iso`, 1431371776 bytes),
  so the app went straight to the main menu: started in 7 frames at 0.31 s, loading screen down at 0.48 s, menu
  previews baked at ~1.8 s. No no-disc page this time, so cc99423f's gate (pad hidden while the no-disc page waits)
  never came into it. Showing no no-disc page was correct here.
- No error, panic, or touch-related warning in the log; no "Skipped event for unknown winit window". The menus
  redrew at full rate from 2.9 s to 7.7 s (something was moving: touches or the menu's own intro), then sat still
  (calm idle, 30 fps) until 60.5 s, then the app went to the background (Suspended at 60.7 s). The app stayed alive
  and rendering the whole time.
- The log can't say what the touches did: no touch logging existed (B739's suggested log lines never landed). The
  1.8.1 log from B739 (`context/iphone-touch/run1`) has the same shape.

## What can leave the pad dead (code reading)

- The pad and the menus are fine when every touch's end arrives (B739 showed this on desktop, and the tests do).
- Bevy's `Touches` keeps a touch in `pressed` until its Ended/Cancelled arrives. If iOS loses that end (fingers
  spammed while the window and view are coming up), the touch stays "down" for ever:
  - on a button: the pad's "a second finger on a button neither presses it again" rule (B165/B274) makes that
    button dead from then on;
  - on the stick: "one stick finger at a time" makes the stick dead;
  - off the pad: the menus' pointer (`main_menu/pointer.rs`) follows one finger at a time and would follow the
    stuck one, ignoring every later tap.
  That matches "buttons show, nothing responds". It isn't proven on the phone: no log showed it.

## Fix (crates/hst-play/src/play/touch_pad.rs)

1. **Launch settle** (the user's idea): after the app starts, the pad takes no touches until it has been on show
   with no loading screen over it for 1.5 s (`SETTLE`). Until then every new touch is taken back from `Touches`'
   just-pressed, so neither the pad nor the menus ever take it. Those touches go nowhere for their whole life, even
   if their end never comes. Once armed it stays armed (loading screens already hush input themselves, `mode::hush`).
2. **Lost-finger self-heal**: each finger on a control records the frame it last moved. A new finger landing on a
   button or the stick held only by fingers that haven't moved for 60 frames (`QUIET`: 0.5 s at 120 Hz, 1 s at 60)
   counts those as lost and takes over. The button goes up for one frame and down again, so it registers as a fresh
   press. A real thumb never shares a button or the stick with another finger for that long, so normal play is
   unchanged (two quick fingers on ✕ still press once).
3. **Diagnostics** for the next report: the first 60 touch events after start are logged to hst-log.txt with what
   the pad did with each (ignored as too soon, pressed button N, took the stick, to the menus, end/cancel). The
   moment the pad arms is logged with how many touches it ignored and how many fingers are still down. Each
   lost-finger takeover is logged too.

Tests: `a_lost_finger_gives_way`, `launch_touches_go_nowhere` (an App: early touches reach neither the pad nor the
menus, stay ignored after arming, and new ones work); existing touch-pad tests still pass.

## Not verified / not 1:1

- Not run on the iPhone: no way to tap the phone from here (no XCTest / developer disk image), and the build has to
  go out in a release. The root cause (iOS dropping a touch's end) is inferred from the code, not seen in a log. If
  the pad still goes dead in the next version, its hst-log.txt now shows each early touch and what the pad did with it.
- The menus' pointer (`main_menu/pointer.rs`, not this task's file) still follows a stuck finger if one is lost
  *after* the settle; only the pad heals itself. The launch case is covered by the settle.
- Not a gameplay port; nothing to match against the original.
