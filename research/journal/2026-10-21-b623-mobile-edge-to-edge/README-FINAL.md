# B623 — Mobile Auto aspect: full screen + safe area — FINAL

## What looked bad

B608 (in v1.3.0) already drew the 3D picture under the notch / island / home indicator and kept the HUD and menus in
the safe rect. But the HUD's camera draws only inside that rect, so every menu (whose walls are UI) stopped at it:
shots at 852×393 with `HST_SAFE=59,0,59,21` (iPhone 15 Pro sideways) showed the window's grey clear colour in 59 pt
strips left and right and 21 pt below on the title, main menu, character select and settings (`context/b623/before`).
On Android below 15 the window wasn't edge to edge either: the theme hides only the status bar, the window stopped
above the navigation bar while the root insets still counted it (HUD pushed in twice), and the bar stayed on screen.
iOS kept the home indicator showing.

## Fix

- `widescreen::Bleed` (hst-base): a UI node (percent of the view) whose edges on the view's edge run on to the
  picture's edges (`bleed_out`). `hud_gamma` has a second camera (order 49, same HUD image, on only while the view
  is smaller than the picture) that clears the image and draws copies of those nodes on the whole picture; the HUD's
  camera then draws over it without clearing. So the strips get the walls, inside the view nothing changes, and the
  z order inside the view stays the HUD's own.
- Menus (`main_menu.rs`): every pool slot is a `Bleed`, on for `Span` quads of solid/sheet/court-picture art (not text,
  glyphs, icons: those would stretch). The quad itself is also extended (clipped by the viewport) so tiled walls keep
  their phase across the seam. The splash's court pictures are cropped for the picture's aspect, not the view's.
- The key legend's strip (`legend.rs`) and the loading cover (`mode.rs`) bleed too (above the walls).
- iOS (`widescreen::phone_window`, used by `main.rs`): home indicator auto-hidden, status bar off, edge swipes
  deferred (a second swipe takes them, as games do).
- Android (`HstActivity`): edge to edge (`setDecorFitsSystemWindows(false)` / layout flags) and immersive (system
  bars hidden, swipe shows them transiently), re-applied on focus. The insets read in `android.rs` are then just the
  cutout (hidden bars report 0).
- HUD in a match is unchanged: still inset to the safe rect (`context/b623/r4/match.png`).

Shots (`context/b623/r3`, `r4`): title, main, chars, settings sideways, main in portrait (393×852, 0/59/0/34): walls,
title bar and legend strip reach every edge, all text and buttons inside the safe rect; a 1280×720 window without
insets is as before. Unit test `walls_bleed_to_the_picture`; the Java compiles against android-36 (`javac`).

## Not verified / not 1:1

- No real iPhone or Android phone: insets faked with `HST_SAFE`; the iOS window flags and the Android immersive /
  edge-to-edge code are untested on devices (no device or emulator run in this task). Whether Android reports
  transient (swiped-in) bars as insets is from the docs, not seen: if it does, the HUD shifts while they show.
- The loading screen's court picture stays inside the safe rect (its node is sized in Vw/Vh, which the copy doesn't
  follow); the strips beside it are black. Not shot (no loading frame in a `--shot`).
- During a menu slide a sliding wall/bar leaves the view's edge for a moment and its strip copy hides until it's back.
- The match shot in this setup draws no court (as noted in B608); HUD placement checked only.
- Menus aren't meant to be 1:1 with the original (PLAN.md); nothing here is gameplay.
