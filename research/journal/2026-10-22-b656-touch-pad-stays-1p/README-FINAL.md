# B656 — the on-screen pad stays when it is 1P

## Change
- `hst-base/src/seats.rs`: a flag, `hold_touch` / `touch_held`: the on-screen pad holds a seat. In the browser,
  setting it also tells the page (`web::touch_held` → `hstTouchHeld`).
- `hst/src/play/main_menu/touch_seat.rs`: each menu frame (after `step`) sets the flag. It is on when a connected
  on-screen pad (`emote::is_touch`) is the splash's 1P (`Menu::p1`) or sits in one of the menus' seats. The match the
  menus start keeps the last value; a `--pads` match without the menus has it off, as before.
- `main_menu.rs` `step`: with the flag on, a real pad's first press no longer takes 1P from the screen
  (`pad_takes_touch_seat`, B247/B516). It is then like any other pad: it joins as the next player on the screens that
  seat players (controllers, select, lobby).
- `hst-play/src/play/touch_pad.rs` (iPhone/Android): with the flag on, the pad stays connected and shown beside a
  real one. Once hidden it doesn't come back while a real pad is in. A hidden pad holds no seat, so the flag goes off.
- `web/touch.js`: `getGamepads()` lists the touch pad beside real pads while it is held. `update` keeps the controls
  shown then. A tap-only 1P (the keyboard's seat) or no seat → hides as before.

## Check
- `touch_pad::tests::a_seated_touch_pad_stays_beside_a_real_pad`: held → both gamepads present and the pad shown.
  Let go → gone. Held again → stays gone. The B247 test still passes (it takes a lock, since the flag is global).
- `touch_seat::tests::held_by_the_splash_or_a_seat`.
- Headless Chromium page (scratchpad) stubbing `navigator.getGamepads` under touch.js `?touch`. Touch only → shown.
  Held + Xbox in → both listed, still shown after `gamepadconnected`. Let go → Xbox only, hidden. Held again → stays
  hidden. Pad out → touch back. Not held + Xbox in → Xbox only (as B247).
- `tools/webcheck.sh` clean.

## Not verified / not 1:1
- Not tried on a real phone with a Bluetooth controller (no device here). Human check: on a phone, press Start on the
  touch pad at the splash, connect a controller. The pad should stay on screen and drive 1P. The controller's ✕ on the
  controllers screen or select should join as 2P.
- In a match the seats are fixed (who is human doesn't change mid-match). A controller connecting mid-match drives a
  seat only if one is waiting for a pad, or after the pause screen's controller assignment gives it one. It doesn't
  turn the 2P computer player human by itself.
- If 1P took the splash by tapping the screen (the keyboard's seat) instead of the touch pad's Start, the touch pad
  holds no seat and hides as before. The task line's condition is "the touch pad pressed Start".
- Not 1:1: none (front-end/input shell, not gameplay).
