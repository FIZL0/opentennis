# B752 — iPhone: on-screen pad dead after tapping at start-up

## Reproduced on the iPhone 15 Pro

- Dev build of 7797278 (`tools/ios-ipa.sh`), signed and installed with iFizBox's `tools/ui-15.sh install` (zsign,
  the rig's ad hoc profile, bundle id kept). The ISO went in with `afcclient -u <udid> --container
  io.github.fizl0.hst` `put` (go-ios `file push` refuses files over 1 GB). Taps through WebDriverAgent's `/actions`
  (a small client in the scratchpad: single taps, stick drags, random 1-4 finger spam); screenshots with
  `ios screenshot`; the app's `Documents/hst-log.txt` pulled with afcclient (local copies in `context/b752/`).
- **No spam needed.** Launch, one tap on "Tap to start", then ✕ or the stick on the pad: nothing happens, for good.
  The log shows the pad did its part each time (`touch … presses button 0`, `takes the stick`, every end arrives):
  no touch is lost, the menus throw the presses away.

## Cause

The splash's tap-anywhere gives 1P to the screen's taps (the keyboard's seat, `Dev::Keys`, B748's change: on a
phone the splash is always taken that way now, and before B748 whenever a tap rather than the pad's ✕ took it, as
spammed taps at launch do). The on-screen pad is a gamepad, so to the menus it is "another pad": `Menu::step` drops
every press from a pad that isn't 1P's device outside the seating screens. The hand-over meant for this
(`pad_takes_touch_seat`, B656) is skipped where the takeover prompt may ask (`takeover_here`), which includes the
main menu, and the prompt itself never asks the on-screen pad (it counts as the keyboard there). So the pad stayed
locked out, and the match's 1P seat stayed the keyboard's too. B746/B748's lost-touch theory was never the cause.

## Fix

`main_menu/touch_seat.rs` `Menu::touch_pad_takes_taps`, called from the menu's step loop for any device's press:
a press on the on-screen pad (`takeover.touch`) takes 1P and the seat that a tap took (`Dev::Keys`), unless a real
pad's takeover prompt is up (the pad answers that as the screen's). Taps keep working afterwards: with 1P on a
connected pad they count as that pad's (`pointer::device`), the state the app was in when the pad's ✕ took the
splash. The pad then holds a seat, so B656's "stays beside a controller" applies again.

Test: `takeover::tests::the_on_screen_pad_takes_what_a_tap_took` (the drop before, the hand-over, the stick moving
the main menu's hand after, a real pad and the open prompt left alone, a keyboard seat elsewhere left alone).

## Proven on the phone (fixed build)

- The exact failing sequence (launch, one splash tap, ✕): ✕ now opens Local → character select, 1P "Controller 1".
- Random 1-4 finger spam over the whole screen from launch for 8 s, and middle-only spam (1P taken by taps) for
  6 s: afterwards every control worked: stick (main menu up/down, select left/right), ✕ (choose), ○ (back, out to
  the splash), L1/R1 (Settings tabs "Page"), Start (Mods' reload popup), □ (Singles/Doubles), △ (favourite) and
  Select (hand swap) on the character select.

## Not verified / not 1:1

- WebDriverAgent's spam is slower than a thumb (~2 actions a second, up to 4 fingers each), so "very fast" tapping
  is approximated; the cause doesn't depend on speed (one tap is enough).
- Not tried in a match on the phone (the menus' seat now is the pad's, which is what a match reads; unit-tested
  only through the menus).
- The browser's touch pad (`web/touch.js`) goes through the same menu code and should be fixed by the same change;
  not run in a phone browser.
- Front-end behaviour: ours, nothing to match in the original.
