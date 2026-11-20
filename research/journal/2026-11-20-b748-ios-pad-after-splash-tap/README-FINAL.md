# B748 iOS/touch: no on-screen pad until the splash is tapped

User 2026-11-20: don't show or hook the on-screen controls until the first tap on the splash; that tap only counts in
the middle 75% of the screen, so a grip touch on an edge or corner doesn't start the game.

## What changed

- `touch_pad.rs`: a `Tapped` resource (init false). Until it's set, `drive` returns before spawning the pad's gamepad
  entity: no gamepad connects, nothing is drawn (`draw` spawns on first show), no touch is read or taken back from
  `Touches`, so every touch reaches the menus (the splash). A finger already down when the pad comes holds nothing
  (only new presses land on controls).
- B746's 1.5 s settle gate (`Launch::gate`, `SETTLE`) removed: the pad no longer exists during launch, which was what
  the gate protected. Kept: B746's lost-finger release (`QUIET`, `Fingers::settle`) and the first-touches log.
- `main_menu/splash.rs` `tick`: sets `Tapped` once 1P is taken on the splash or the menu isn't on the splash at all
  (`HST_MENU` screens, a menu kept after a match). `main.rs`: set at start for runs with no menus (`--play`, etc.).
- `main_menu/pointer.rs`: while the splash waits (`Menu::splash_open`: no 1P, no Quit / Update popup), a finger's tap
  that hits no rect only falls back to the splash's tap-anywhere if it started in the middle 75% of the whole screen
  each way (`SPLASH_MIDDLE`). Rects (the update card) still take taps anywhere. Mouse clicks unchanged.
- Release notes: new line; B746's unreleased line no longer claims the startup ignore window.

Tests: `touch_pad::no_pad_till_the_splash_is_tapped`, `pointer::splash_tap_only_in_the_middle`; the other touch_pad
tests set `Tapped(true)`.

## Consequence

B656 (the pad keeps 1P beside a controller) relied on the pad pressing Start on the splash; now the splash's 1P on a
phone is always the screen's tap (`Dev::Keys`), so a controller connected later replaces the pad as before B656
unless the pad takes a seat in the menus. Matches the ask (the pad isn't there on the splash).

## Not verified / not 1:1

- Not run on a phone (no device in an unattended run); covered by unit tests of the pad gate and the middle test.
  Front-end behaviour: ours, nothing to match in the original.
- The browser's pad (`web/touch.js`) is unchanged; the middle-75% rule applies to its touches too since they go
  through the same pointer system.
